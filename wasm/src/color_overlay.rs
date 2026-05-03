// Color overlay blending (normal / multiply / screen / soft-light) with
// optional vignette darkening at image corners.

#[inline]
fn multiply(a: f32, b: f32) -> f32 {
    a * b
}

#[inline]
fn screen(a: f32, b: f32) -> f32 {
    1.0 - (1.0 - a) * (1.0 - b)
}

#[inline]
fn soft_light(base: f32, blend: f32) -> f32 {
    if blend <= 0.5 {
        base - (1.0 - 2.0 * blend) * base * (1.0 - base)
    } else {
        let d = if base <= 0.25 {
            ((16.0 * base - 12.0) * base + 4.0) * base
        } else {
            base.sqrt()
        };
        base + (2.0 * blend - 1.0) * (d - base)
    }
}

/// Apply a solid-color overlay to `pixels` using the specified blend mode and alpha.
///
/// `blend_mode`: `"normal"` | `"multiply"` | `"screen"` | `"softlight"`
/// `vignette`: 0.0 = off, 1.0 = strong dark vignette at corners
#[allow(clippy::too_many_arguments)]
pub fn apply_color_overlay(
    pixels: &[u8],
    width: u32,
    height: u32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
    blend_mode: &str,
    vignette: f32,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let alpha = alpha.clamp(0.0, 1.0);
    let vignette = vignette.clamp(0.0, 1.0);
    let cr = r as f32 / 255.0;
    let cg = g as f32 / 255.0;
    let cb = b as f32 / 255.0;

    let blend_fn: fn(f32, f32) -> f32 = match blend_mode {
        "multiply" => multiply,
        "screen" => screen,
        "softlight" => soft_light,
        _ => |_base, _overlay| _overlay, // "normal"
    };

    let cx = (w as f32 - 1.0) / 2.0;
    let cy = (h as f32 - 1.0) / 2.0;
    let max_dist = (cx * cx + cy * cy).sqrt();

    let n = w * h;
    let mut out = pixels.to_vec();

    for i in 0..n {
        let pr = pixels[i * 4] as f32 / 255.0;
        let pg = pixels[i * 4 + 1] as f32 / 255.0;
        let pb = pixels[i * 4 + 2] as f32 / 255.0;

        // Blend color overlay
        let br = blend_fn(pr, cr);
        let bg = blend_fn(pg, cg);
        let bb = blend_fn(pb, cb);

        let mut or_ = pr + (br - pr) * alpha;
        let mut og = pg + (bg - pg) * alpha;
        let mut ob = pb + (bb - pb) * alpha;

        // Vignette: darken corners by multiplying with a radial weight
        if vignette > 1e-6 {
            let px = (i % w) as f32;
            let py = (i / w) as f32;
            let dx = px - cx;
            let dy = py - cy;
            let dist_norm = (dx * dx + dy * dy).sqrt() / max_dist; // 0..1
                                                                   // Smooth vignette falloff
            let vig_weight = 1.0 - vignette * dist_norm * dist_norm;
            let vig_weight = vig_weight.max(0.0);
            or_ *= vig_weight;
            og *= vig_weight;
            ob *= vig_weight;
        }

        out[i * 4] = (or_ * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        out[i * 4 + 1] = (og * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        out[i * 4 + 2] = (ob * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        out[i * 4 + 3] = pixels[i * 4 + 3];
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_image(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut p = Vec::with_capacity(w * h * 4);
        for _ in 0..(w * h) {
            p.extend_from_slice(&[r, g, b, 255]);
        }
        p
    }

    #[test]
    fn zero_alpha_is_identity() {
        let p = solid_image(10, 10, 100, 150, 200);
        let out = apply_color_overlay(&p, 10, 10, 255, 0, 0, 0.0, "normal", 0.0);
        assert_eq!(out, p);
    }

    #[test]
    fn normal_blend_at_full_alpha() {
        let p = solid_image(4, 4, 100, 100, 100);
        let out = apply_color_overlay(&p, 4, 4, 200, 200, 200, 1.0, "normal", 0.0);
        for i in (0..out.len()).step_by(4) {
            assert!((out[i] as i32 - 200).abs() <= 1);
        }
    }

    #[test]
    fn multiply_blend_white_is_identity() {
        let p = solid_image(4, 4, 120, 180, 60);
        // Multiply with white (255,255,255) at alpha=1 should be identity
        let out = apply_color_overlay(&p, 4, 4, 255, 255, 255, 1.0, "multiply", 0.0);
        for i in (0..out.len()).step_by(4) {
            assert!(
                (out[i] as i32 - p[i] as i32).abs() <= 1,
                "ch0 at {i}: {} vs {}",
                out[i],
                p[i]
            );
        }
    }

    #[test]
    fn vignette_darkens_corners() {
        let p = solid_image(64, 64, 200, 200, 200);
        let out = apply_color_overlay(&p, 64, 64, 0, 0, 0, 0.0, "normal", 1.0);
        let center = (32 * 64 + 32) * 4;
        let corner = 0;
        assert!(
            out[center] > out[corner],
            "center should be brighter than corner"
        );
    }

    #[test]
    fn alpha_preserved() {
        let p: Vec<u8> = (0..16).flat_map(|_| [100u8, 100, 100, 128]).collect();
        let out = apply_color_overlay(&p, 2, 2, 255, 0, 0, 0.5, "normal", 0.0);
        for i in (3..out.len()).step_by(4) {
            assert_eq!(out[i], 128);
        }
    }

    #[test]
    fn screen_blend_black_is_identity() {
        // Screen with black (0,0,0) at any alpha → no change.
        // screen(dst, 0) = 1 - (1-dst)*1 = dst
        let p = solid_image(4, 4, 120, 80, 200);
        let out = apply_color_overlay(&p, 4, 4, 0, 0, 0, 1.0, "screen", 0.0);
        for i in (0..out.len()).step_by(4) {
            assert!(
                (out[i] as i32 - p[i] as i32).abs() <= 1,
                "R mismatch at {i}: {} vs {}",
                out[i],
                p[i]
            );
        }
    }

    #[test]
    fn softlight_mid_grey_is_near_identity() {
        // Soft-light with 50% grey overlay produces almost-identity.
        let p = solid_image(4, 4, 100, 150, 200);
        let out = apply_color_overlay(&p, 4, 4, 128, 128, 128, 1.0, "softlight", 0.0);
        for i in (0..out.len()).step_by(4) {
            let d = (out[i] as i32 - p[i] as i32).abs();
            assert!(d <= 10, "ch R drifted by {} at pixel {}", d, i / 4);
        }
    }
}
