// Background processing using a foreground segmentation mask.
//
// Modes:
//   "blur"  – Gaussian-approximate blur of background via wgpu (CPU box-blur fallback)
//   "solid" – replace background with a flat colour
//   "image" – replace background with a supplied replacement image (RGBA)
//
// The segmentation mask is a Float32Array (f32 per pixel, 1.0 = foreground,
// 0.0 = background) produced by the MediaPipe ImageSegmenter on the JS side.

use crate::gpu::with_gpu;
use crate::skin_mask::box_blur_pass;
use wasm_bindgen::prelude::*;

// ── CPU helpers ───────────────────────────────────────────────────────────────

/// Separable 3-pass box blur ≈ Gaussian on a single channel.
fn box_blur_channel(src: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    if radius == 0 {
        return src.to_vec();
    }
    let mut a = src.to_vec();
    let mut b = vec![0u8; src.len()];
    for _ in 0..3 {
        box_blur_pass(&a, &mut b, width, height, radius);
        std::mem::swap(&mut a, &mut b);
    }
    a
}

/// Blur each RGBA channel independently with the same box-blur radius.
fn blur_rgba_cpu(pixels: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    let n = width * height;
    // Extract each channel into flat slices.
    let mut r_ch = vec![0u8; n];
    let mut g_ch = vec![0u8; n];
    let mut b_ch = vec![0u8; n];
    for i in 0..n {
        r_ch[i] = pixels[i * 4];
        g_ch[i] = pixels[i * 4 + 1];
        b_ch[i] = pixels[i * 4 + 2];
    }
    let r_blurred = box_blur_channel(&r_ch, width, height, radius);
    let g_blurred = box_blur_channel(&g_ch, width, height, radius);
    let b_blurred = box_blur_channel(&b_ch, width, height, radius);

    let mut out = pixels.to_vec();
    for i in 0..n {
        out[i * 4] = r_blurred[i];
        out[i * 4 + 1] = g_blurred[i];
        out[i * 4 + 2] = b_blurred[i];
    }
    out
}

// ── GPU blur ──────────────────────────────────────────────────────────────────

// One-pass separable box blur (horizontal OR vertical) dispatched as a compute
// shader. Called 6 times total (H+V ×3) for a Gaussian approximation.

const BLUR_H_SHADER: &str = r#"
struct Params { width: u32, height: u32, radius: u32, };
@group(0) @binding(0) var<storage, read>       src:    array<u32>;
@group(0) @binding(1) var<storage, read_write> dst:    array<u32>;
@group(0) @binding(2) var<uniform>             params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x; let y = gid.y;
    if x >= params.width || y >= params.height { return; }
    var acc = vec4<f32>(0.0);
    let r = i32(params.radius);
    for (var dx = -r; dx <= r; dx++) {
        let sx = clamp(i32(x) + dx, 0, i32(params.width) - 1);
        let px = src[y * params.width + u32(sx)];
        acc += vec4<f32>(f32(px & 0xFFu), f32((px >> 8u) & 0xFFu),
                         f32((px >> 16u) & 0xFFu), f32((px >> 24u) & 0xFFu));
    }
    let win = f32(2 * r + 1);
    let out = vec4<u32>(
        u32(clamp(acc.r / win + 0.5, 0.0, 255.0)),
        u32(clamp(acc.g / win + 0.5, 0.0, 255.0)),
        u32(clamp(acc.b / win + 0.5, 0.0, 255.0)),
        u32(clamp(acc.a / win + 0.5, 0.0, 255.0)),
    );
    dst[y * params.width + x] = out.r | (out.g << 8u) | (out.b << 16u) | (out.a << 24u);
}
"#;

const BLUR_V_SHADER: &str = r#"
struct Params { width: u32, height: u32, radius: u32, };
@group(0) @binding(0) var<storage, read>       src:    array<u32>;
@group(0) @binding(1) var<storage, read_write> dst:    array<u32>;
@group(0) @binding(2) var<uniform>             params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x; let y = gid.y;
    if x >= params.width || y >= params.height { return; }
    var acc = vec4<f32>(0.0);
    let r = i32(params.radius);
    for (var dy = -r; dy <= r; dy++) {
        let sy = clamp(i32(y) + dy, 0, i32(params.height) - 1);
        let px = src[u32(sy) * params.width + x];
        acc += vec4<f32>(f32(px & 0xFFu), f32((px >> 8u) & 0xFFu),
                         f32((px >> 16u) & 0xFFu), f32((px >> 24u) & 0xFFu));
    }
    let win = f32(2 * r + 1);
    let out = vec4<u32>(
        u32(clamp(acc.r / win + 0.5, 0.0, 255.0)),
        u32(clamp(acc.g / win + 0.5, 0.0, 255.0)),
        u32(clamp(acc.b / win + 0.5, 0.0, 255.0)),
        u32(clamp(acc.a / win + 0.5, 0.0, 255.0)),
    );
    dst[y * params.width + x] = out.r | (out.g << 8u) | (out.b << 16u) | (out.a << 24u);
}
"#;

async fn blur_rgba_gpu(pixels: &[u8], width: u32, height: u32, radius: u32) -> Option<Vec<u8>> {
    use wgpu::util::DeviceExt;

    with_gpu(|ctx| {
        let device = &ctx.device;
        let queue = &ctx.queue;
        let w = width as usize;
        let h = height as usize;
        let n = w * h;

        let packed: Vec<u32> = (0..n)
            .map(|i| {
                (pixels[i * 4] as u32)
                    | ((pixels[i * 4 + 1] as u32) << 8)
                    | ((pixels[i * 4 + 2] as u32) << 16)
                    | ((pixels[i * 4 + 3] as u32) << 24)
            })
            .collect();

        let usage_rw = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST;

        let mut buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blur_a"),
            contents: bytemuck::cast_slice(&packed),
            usage: usage_rw,
        });
        let mut buf_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur_b"),
            size: (n * 4) as u64,
            usage: usage_rw,
            mapped_at_creation: false,
        });

        let uniforms: [u32; 3] = [width, height, radius];
        let uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blur_uni"),
            contents: bytemuck::cast_slice(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let shader_h = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_h"),
            source: wgpu::ShaderSource::Wgsl(BLUR_H_SHADER.into()),
        });
        let shader_v = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_v"),
            source: wgpu::ShaderSource::Wgsl(BLUR_V_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur_pll"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline_h = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("blur_h_pl"),
            layout: Some(&pl_layout),
            module: &shader_h,
            entry_point: "main",
            compilation_options: Default::default(),
        });
        let pipeline_v = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("blur_v_pl"),
            layout: Some(&pl_layout),
            module: &shader_v,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let wx = width.div_ceil(8);
        let wy = height.div_ceil(8);

        // 3 × (H + V) passes for Gaussian approximation
        for _ in 0..3 {
            for (pipeline, read_buf, write_buf) in
                [(&pipeline_h, &buf_a, &buf_b), (&pipeline_v, &buf_b, &buf_a)]
            {
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("blur_bg"),
                    layout: &bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: read_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: write_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: uni_buf.as_entire_binding(),
                        },
                    ],
                });
                let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("blur_enc"),
                });
                {
                    let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("blur_pass"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.dispatch_workgroups(wx, wy, 1);
                }
                queue.submit(std::iter::once(enc.finish()));
            }
        }

        // After 6 passes (H+V×3), result is in buf_a
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur_stage"),
            size: (n * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("blur_copy"),
        });
        enc.copy_buffer_to_buffer(&buf_a, 0, &staging, 0, (n * 4) as u64);
        queue.submit(std::iter::once(enc.finish()));

        let buf_slice = staging.slice(..);
        buf_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        let mapped = buf_slice.get_mapped_range();
        let packed_out: &[u32] = bytemuck::cast_slice(&mapped);
        let mut out = vec![0u8; n * 4];
        for (i, &p) in packed_out.iter().enumerate() {
            out[i * 4] = (p & 0xFF) as u8;
            out[i * 4 + 1] = ((p >> 8) & 0xFF) as u8;
            out[i * 4 + 2] = ((p >> 16) & 0xFF) as u8;
            out[i * 4 + 3] = ((p >> 24) & 0xFF) as u8;
        }
        drop(mapped);
        staging.unmap();

        // Swap mutable references for borrow-checker: return `out`
        let _ = (&mut buf_b, &mut buf_a);
        out
    })
}

// ── Composite foreground + blurred/replaced background ───────────────────────

fn composite(
    original: &[u8],
    processed_bg: &[u8],
    seg_mask: &[f32],
    width: usize,
    height: usize,
) -> Vec<u8> {
    let n = width * height;
    let mut out = vec![0u8; n * 4];
    for i in 0..n {
        // seg_mask[i] = 1 → foreground (keep original), 0 → background (use processed_bg)
        let fg = seg_mask.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        let bg = 1.0 - fg;
        for c in 0..4 {
            let o = original[i * 4 + c] as f32;
            let p = processed_bg[i * 4 + c] as f32;
            out[i * 4 + c] = (o * fg + p * bg + 0.5) as u8;
        }
    }
    out
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Process the background using the provided segmentation mask.
///
/// - `mode`: `"blur"` | `"solid"` | `"image"`
/// - `seg_mask`: per-pixel foreground probability, `len == width * height`
/// - `repl_r/g/b`: colour for "solid" mode
/// - `repl_pixels`: RGBA replacement image for "image" mode (length == pixels.length or empty)
/// - `blur_radius`: blur radius in pixels (used for "blur" mode)
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub async fn process_background(
    pixels: &[u8],
    width: u32,
    height: u32,
    seg_mask: &[f32],
    mode: &str,
    repl_r: u8,
    repl_g: u8,
    repl_b: u8,
    repl_pixels: &[u8],
    blur_radius: u32,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let n = w * h;

    if seg_mask.len() < n {
        return pixels.to_vec();
    }

    match mode {
        "blur" => {
            let blurred =
                if let Some(gpu_out) = blur_rgba_gpu(pixels, width, height, blur_radius).await {
                    gpu_out
                } else {
                    blur_rgba_cpu(pixels, w, h, blur_radius as usize)
                };
            composite(pixels, &blurred, seg_mask, w, h)
        }
        "solid" => {
            let solid: Vec<u8> = (0..n)
                .flat_map(|_| [repl_r, repl_g, repl_b, 255u8])
                .collect();
            composite(pixels, &solid, seg_mask, w, h)
        }
        "image" if repl_pixels.len() >= n * 4 => composite(pixels, repl_pixels, seg_mask, w, h),
        _ => pixels.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_rgba(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        (0..w * h).flat_map(|_| [r, g, b, 255]).collect()
    }

    #[test]
    fn blur_cpu_smooths_edge() {
        let w = 20usize;
        let h = 10usize;
        // Left half red, right half blue
        let mut pixels = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                if x < w / 2 {
                    pixels[i] = 255; // R
                } else {
                    pixels[i + 2] = 255; // B
                }
                pixels[i + 3] = 255;
            }
        }
        let blurred = blur_rgba_cpu(&pixels, w, h, 2);
        // The pixel at the boundary should be mixed
        let mid = (5 * w + w / 2) * 4;
        assert!(
            blurred[mid] > 0 && blurred[mid] < 255,
            "boundary R: {}",
            blurred[mid]
        );
        assert!(
            blurred[mid + 2] > 0 && blurred[mid + 2] < 255,
            "boundary B: {}",
            blurred[mid + 2]
        );
    }

    #[test]
    fn composite_full_fg_is_original() {
        let w = 4usize;
        let h = 4usize;
        let orig = solid_rgba(w, h, 200, 100, 50);
        let bg = solid_rgba(w, h, 0, 0, 255);
        let mask: Vec<f32> = vec![1.0; w * h]; // all foreground
        let out = composite(&orig, &bg, &mask, w, h);
        for i in (0..out.len()).step_by(4) {
            assert_eq!(out[i], 200);
            assert_eq!(out[i + 1], 100);
            assert_eq!(out[i + 2], 50);
        }
    }

    #[test]
    fn composite_full_bg_is_replaced() {
        let w = 4usize;
        let h = 4usize;
        let orig = solid_rgba(w, h, 200, 100, 50);
        let bg = solid_rgba(w, h, 30, 40, 50);
        let mask: Vec<f32> = vec![0.0; w * h]; // all background
        let out = composite(&orig, &bg, &mask, w, h);
        for i in (0..out.len()).step_by(4) {
            assert_eq!(out[i], 30);
        }
    }

    #[test]
    fn solid_mode_replaces_background() {
        let w = 4usize;
        let h = 4usize;
        // All background (mask=0.0) → pixel becomes the solid colour.
        let orig = solid_rgba(w, h, 200, 100, 50);
        let mask: Vec<f32> = vec![0.0; w * h];
        let solid: Vec<u8> = (0..w * h).flat_map(|_| [10u8, 20, 30, 255]).collect();
        let out = composite(&orig, &solid, &mask, w, h);
        for i in (0..out.len()).step_by(4) {
            assert_eq!(out[i], 10, "R mismatch at {}", i / 4);
            assert_eq!(out[i + 1], 20, "G mismatch at {}", i / 4);
            assert_eq!(out[i + 2], 30, "B mismatch at {}", i / 4);
        }
    }

    #[test]
    fn image_mode_blends_replacement() {
        let w = 4usize;
        let h = 4usize;
        let orig = solid_rgba(w, h, 200, 100, 50);
        let repl = solid_rgba(w, h, 0, 0, 128);
        // mask=0.0 at first pixel → should get replacement colour
        let mut mask = vec![1.0f32; w * h];
        mask[0] = 0.0;
        let out = composite(&orig, &repl, &mask, w, h);
        assert_eq!(out[0], 0, "replaced R");
        assert_eq!(out[2], 128, "replaced B");
        assert_eq!(out[4], 200, "fg R unchanged");
    }

    #[test]
    fn solid_mode_feathers_at_midmask() {
        // mask=0.5 → output should be midpoint between fg and bg colour.
        let w = 2usize;
        let h = 2usize;
        let orig = solid_rgba(w, h, 200, 200, 200);
        let bg = solid_rgba(w, h, 0, 0, 0);
        let mask = vec![0.5f32; w * h];
        let out = composite(&orig, &bg, &mask, w, h);
        for i in (0..out.len()).step_by(4) {
            let v = out[i] as i32;
            assert!(
                (v - 100).abs() <= 2,
                "expected ~100 at {}, got {}",
                i / 4,
                v
            );
        }
    }
}
