// 3D Look-Up Table colour grading with 7 built-in プリクラ presets.
//
// LUT dimensions: 17 × 17 × 17 (R × G × B grid), u8 encoded.
// GPU path: 3-D texture sampled with a linear sampler in a compute shader —
//   each pixel's (R,G,B) becomes the 3-D texture coordinate, giving GPU-native
//   trilinear interpolation in a single dispatch.
// CPU fallback: manual trilinear interpolation (≈ 20 ms @ 640×480).
//
// Preset design follows common プリクラ aesthetics:
//   natural   – 微warmth, subtle S-curve contrast
//   pop       – high saturation, bright highlights (classic プリクラ)
//   soft      – matte: lifted blacks, muted chroma
//   film      – Kodak Portra style: warm highlights, cool shadows
//   vintage   – faded, sepia-tinted, nostalgic
//   cool      – blue-purple cast, cool-beauty style
//   peach     – warm peach/salmon for flattering skin tones

use crate::gpu::with_gpu;
use wasm_bindgen::prelude::*;

const LUT_DIM: usize = 17;
const LUT_SIZE: usize = LUT_DIM * LUT_DIM * LUT_DIM;

// ── Colour math helpers ───────────────────────────────────────────────────────

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-7 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-7 {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 1e-7 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 1e-7 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue2rgb = |p: f32, q: f32, mut t: f32| -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 0.5 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };
    (
        hue2rgb(p, q, h + 1.0 / 3.0),
        hue2rgb(p, q, h),
        hue2rgb(p, q, h - 1.0 / 3.0),
    )
}

fn adjust_saturation(r: f32, g: f32, b: f32, factor: f32) -> (f32, f32, f32) {
    let (h, s, l) = rgb_to_hsl(r, g, b);
    hsl_to_rgb(h, (s * factor).clamp(0.0, 1.0), l)
}

fn s_curve(v: f32, strength: f32) -> f32 {
    let s = v * v * (3.0 - 2.0 * v); // smoothstep
    v * (1.0 - strength) + s * strength
}

fn clamp3(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

// ── Preset transforms (f32 → f32, all values in 0..1) ────────────────────────

fn t_natural(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // Warm skin tone, clear S-curve contrast — classic film-like natural look.
    let (r, g, b) = clamp3(r * 1.10 + 0.03, g * 1.03 + 0.01, b * 0.90);
    clamp3(s_curve(r, 0.18), s_curve(g, 0.14), s_curve(b, 0.10))
}

fn t_pop(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // High saturation + brightness boost + strong S-curve contrast (classic プリクラ pop).
    let (r, g, b) = adjust_saturation(r, g, b, 1.9);
    let (r, g, b) = clamp3(r * 1.10 + 0.02, g * 1.10 + 0.02, b * 1.08 + 0.02);
    clamp3(s_curve(r, 0.30), s_curve(g, 0.30), s_curve(b, 0.28))
}

fn t_soft(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // Lifted blacks (matte): compress to [0.06, 0.94] range
    let lift = |v: f32| v * 0.88 + 0.06;
    let (r, g, b) = adjust_saturation(r, g, b, 0.82);
    clamp3(lift(r), lift(g), lift(b))
}

fn t_film(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // Kodak Portra style: warm highlights, cool shadows, slight contrast boost.
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    let warm = lum;
    let cool = 1.0 - lum;
    let (r, g, b) = adjust_saturation(r, g, b, 0.88);
    let (r, g, b) = clamp3(
        r + 0.07 * warm - 0.02 * cool,
        g + 0.02 * warm - 0.01 * cool,
        b - 0.06 * warm + 0.07 * cool,
    );
    clamp3(s_curve(r, 0.10), s_curve(g, 0.08), s_curve(b, 0.06))
}

fn t_vintage(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let lift = |v: f32| v * 0.82 + 0.06;
    let (r, g, b) = adjust_saturation(r, g, b, 0.55);
    // Warm faded cast
    clamp3(lift(r) + 0.04, lift(g) + 0.01, lift(b) - 0.03)
}

fn t_cool(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    let (r, g, b) = adjust_saturation(r, g, b, 1.15);
    clamp3(r - 0.08 + 0.03 * lum, g - 0.02, b + 0.10 - 0.03 * lum)
}

fn t_peach(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    let (r, g, b) = clamp3(r * 1.06 + 0.02, g * 1.01 + 0.01, b * 0.93);
    // Peach tint: add warm pink in highlights
    clamp3(r + 0.03 * lum, g + 0.01 * lum, b - 0.04 * lum)
}

// ── LUT construction ──────────────────────────────────────────────────────────

type Transform = fn(f32, f32, f32) -> (f32, f32, f32);

fn build_lut(transform: Transform) -> Box<[u8]> {
    let mut lut = vec![0u8; LUT_SIZE * 3].into_boxed_slice();
    for ri in 0..LUT_DIM {
        for gi in 0..LUT_DIM {
            for bi in 0..LUT_DIM {
                let r = ri as f32 / (LUT_DIM - 1) as f32;
                let g = gi as f32 / (LUT_DIM - 1) as f32;
                let b = bi as f32 / (LUT_DIM - 1) as f32;
                let (ro, go, bo) = transform(r, g, b);
                let idx = (ri * LUT_DIM * LUT_DIM + gi * LUT_DIM + bi) * 3;
                lut[idx] = (ro * 255.0 + 0.5) as u8;
                lut[idx + 1] = (go * 255.0 + 0.5) as u8;
                lut[idx + 2] = (bo * 255.0 + 0.5) as u8;
            }
        }
    }
    lut
}

fn get_lut(preset: &str) -> Box<[u8]> {
    match preset {
        "natural" => build_lut(t_natural),
        "pop" => build_lut(t_pop),
        "soft" => build_lut(t_soft),
        "film" => build_lut(t_film),
        "vintage" => build_lut(t_vintage),
        "cool" => build_lut(t_cool),
        "peach" => build_lut(t_peach),
        _ => {
            // Identity LUT
            build_lut(|r, g, b| (r, g, b))
        }
    }
}

// ── CPU trilinear lookup ──────────────────────────────────────────────────────

fn trilinear(lut: &[u8], r: f32, g: f32, b: f32) -> (u8, u8, u8) {
    let scale = (LUT_DIM - 1) as f32;
    let ri = (r * scale).clamp(0.0, scale);
    let gi = (g * scale).clamp(0.0, scale);
    let bi = (b * scale).clamp(0.0, scale);

    let r0 = ri.floor() as usize;
    let g0 = gi.floor() as usize;
    let b0 = bi.floor() as usize;
    let r1 = (r0 + 1).min(LUT_DIM - 1);
    let g1 = (g0 + 1).min(LUT_DIM - 1);
    let b1 = (b0 + 1).min(LUT_DIM - 1);
    let rf = ri - r0 as f32;
    let gf = gi - g0 as f32;
    let bf = bi - b0 as f32;

    let sample = |ri: usize, gi: usize, bi: usize, ch: usize| -> f32 {
        lut[(ri * LUT_DIM * LUT_DIM + gi * LUT_DIM + bi) * 3 + ch] as f32
    };

    let ch = |ch: usize| {
        let c000 = sample(r0, g0, b0, ch);
        let c100 = sample(r1, g0, b0, ch);
        let c010 = sample(r0, g1, b0, ch);
        let c110 = sample(r1, g1, b0, ch);
        let c001 = sample(r0, g0, b1, ch);
        let c101 = sample(r1, g0, b1, ch);
        let c011 = sample(r0, g1, b1, ch);
        let c111 = sample(r1, g1, b1, ch);
        // Trilinear
        let r_lo = c000 + rf * (c100 - c000);
        let r_hi = c010 + rf * (c110 - c010);
        let rr_lo = r_lo + gf * (r_hi - r_lo);
        let r_lo2 = c001 + rf * (c101 - c001);
        let r_hi2 = c011 + rf * (c111 - c011);
        let rr_hi = r_lo2 + gf * (r_hi2 - r_lo2);
        rr_lo + bf * (rr_hi - rr_lo)
    };

    (
        ch(0).clamp(0.0, 255.0) as u8,
        ch(1).clamp(0.0, 255.0) as u8,
        ch(2).clamp(0.0, 255.0) as u8,
    )
}

fn apply_lut_cpu(pixels: &[u8], lut: &[u8]) -> Vec<u8> {
    let n = pixels.len() / 4;
    let mut out = pixels.to_vec();
    for i in 0..n {
        let r = pixels[i * 4] as f32 / 255.0;
        let g = pixels[i * 4 + 1] as f32 / 255.0;
        let b = pixels[i * 4 + 2] as f32 / 255.0;
        let (ro, go, bo) = trilinear(lut, r, g, b);
        out[i * 4] = ro;
        out[i * 4 + 1] = go;
        out[i * 4 + 2] = bo;
    }
    out
}

// ── GPU path ──────────────────────────────────────────────────────────────────

const LUT_SHADER: &str = r#"
struct Uniforms {
    n_pixels: u32,
};
@group(0) @binding(0) var<storage, read>       in_pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> out_pixels: array<u32>;
@group(0) @binding(2) var lut_tex: texture_3d<f32>;
@group(0) @binding(3) var lut_smp: sampler;
@group(0) @binding(4) var<uniform>             uniforms: Uniforms;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= uniforms.n_pixels { return; }

    let px = in_pixels[idx];
    let r = f32(px & 0xFFu)       / 255.0;
    let g = f32((px >> 8u)  & 0xFFu) / 255.0;
    let b = f32((px >> 16u) & 0xFFu) / 255.0;
    let a = (px >> 24u) & 0xFFu;

    // 3-D LUT: R→x, G→y, B→z, all in [0..1]
    let mapped = textureSampleLevel(lut_tex, lut_smp, vec3<f32>(r, g, b), 0.0).rgb;

    let ro = u32(clamp(mapped.r * 255.0 + 0.5, 0.0, 255.0));
    let go = u32(clamp(mapped.g * 255.0 + 0.5, 0.0, 255.0));
    let bo = u32(clamp(mapped.b * 255.0 + 0.5, 0.0, 255.0));
    out_pixels[idx] = ro | (go << 8u) | (bo << 16u) | (a << 24u);
}
"#;

async fn apply_lut_gpu(pixels: &[u8], width: u32, height: u32, lut: &[u8]) -> Option<Vec<u8>> {
    use wgpu::util::DeviceExt;

    let result: Option<Vec<u8>> = with_gpu(|ctx| {
        let device = &ctx.device;
        let queue = &ctx.queue;
        let n = (width * height) as usize;

        // Pack RGBA as u32 (little-endian: R=bits0-7, G=bits8-15, B=bits16-23, A=bits24-31)
        let packed: Vec<u32> = (0..n)
            .map(|i| {
                (pixels[i * 4] as u32)
                    | ((pixels[i * 4 + 1] as u32) << 8)
                    | ((pixels[i * 4 + 2] as u32) << 16)
                    | ((pixels[i * 4 + 3] as u32) << 24)
            })
            .collect();

        let in_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lut_in"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lut_out"),
            size: (n * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lut_staging"),
            size: (n * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Build LUT data for 3-D texture: 17x17x17, Rgba8Unorm
        let mut tex_data = vec![0u8; LUT_SIZE * 4];
        for i in 0..LUT_SIZE {
            tex_data[i * 4] = lut[i * 3]; // R
            tex_data[i * 4 + 1] = lut[i * 3 + 1]; // G
            tex_data[i * 4 + 2] = lut[i * 3 + 2]; // B
            tex_data[i * 4 + 3] = 255;
        }

        let lut_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("lut_3d"),
                size: wgpu::Extent3d {
                    width: LUT_DIM as u32,
                    height: LUT_DIM as u32,
                    depth_or_array_layers: LUT_DIM as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &tex_data,
        );
        let lut_view = lut_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
        let lut_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniforms_data: [u32; 1] = [n as u32];
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lut_uni"),
            contents: bytemuck::cast_slice(&uniforms_data),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lut"),
            source: wgpu::ShaderSource::Wgsl(LUT_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lut_bgl"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
            label: Some("lut_pll"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("lut_pipeline"),
            layout: Some(&pl_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lut_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: in_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&lut_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("lut") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lut_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups((n as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, (n * 4) as u64);
        queue.submit(std::iter::once(encoder.finish()));

        // Read back
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
        out
    });

    result
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Apply a named LUT preset to `pixels`. GPU path when WebGPU is initialised;
/// falls back to CPU trilinear interpolation otherwise.
#[wasm_bindgen]
pub async fn apply_lut3d(pixels: &[u8], width: u32, height: u32, preset: &str) -> Vec<u8> {
    if preset == "none" {
        return pixels.to_vec();
    }
    let lut = get_lut(preset);
    if let Some(out) = apply_lut_gpu(pixels, width, height, &lut).await {
        out
    } else {
        apply_lut_cpu(pixels, &lut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8) -> Vec<u8> {
        vec![r, g, b, 255].repeat(16)
    }

    #[test]
    fn identity_lut_preserves_pixels() {
        let lut = build_lut(|r, g, b| (r, g, b));
        let p = solid(128, 64, 200);
        let out = apply_lut_cpu(&p, &lut);
        for i in (0..out.len()).step_by(4) {
            assert!((out[i] as i32 - p[i] as i32).abs() <= 1, "R drift");
            assert!((out[i + 1] as i32 - p[i + 1] as i32).abs() <= 1, "G drift");
            assert!((out[i + 2] as i32 - p[i + 2] as i32).abs() <= 1, "B drift");
        }
    }

    #[test]
    fn alpha_preserved() {
        let p: Vec<u8> = (0..16).flat_map(|_| [100u8, 150, 80, 42]).collect();
        let lut = build_lut(t_pop);
        let out = apply_lut_cpu(&p, &lut);
        for i in (3..out.len()).step_by(4) {
            assert_eq!(out[i], 42);
        }
    }

    #[test]
    fn pop_increases_saturation() {
        // A grey pixel should remain grey (saturation increase of grey = grey).
        let grey = solid(128, 128, 128);
        let lut = build_lut(t_pop);
        let out = apply_lut_cpu(&grey, &lut);
        // All channels must remain close to each other (grey stays grey).
        let r = out[0] as i32;
        let g = out[1] as i32;
        let b = out[2] as i32;
        assert!((r - g).abs() <= 3, "grey drift R-G: {r} vs {g}");
        assert!((r - b).abs() <= 3, "grey drift R-B: {r} vs {b}");
    }

    #[test]
    fn soft_lifts_blacks() {
        let black = solid(0, 0, 0);
        let lut = build_lut(t_soft);
        let out = apply_lut_cpu(&black, &lut);
        assert!(out[0] > 10, "soft should lift black R");
    }

    #[test]
    fn trilinear_boundary_safe() {
        // Pure white / pure black should not panic.
        let lut = build_lut(t_film);
        let white = solid(255, 255, 255);
        let black = solid(0, 0, 0);
        let _ = apply_lut_cpu(&white, &lut);
        let _ = apply_lut_cpu(&black, &lut);
    }

    #[test]
    fn none_preset_is_identity() {
        let p = solid(100, 150, 200);
        let lut = get_lut("none");
        let out = apply_lut_cpu(&p, &lut);
        for i in (0..out.len()).step_by(4) {
            assert!((out[i] as i32 - p[i] as i32).abs() <= 1);
        }
    }
}
