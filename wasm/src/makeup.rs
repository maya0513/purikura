// Makeup effects: lip colour, eye shadow, cheek blush.
//
// Each effect applies a colour blend inside a landmark-based polygon or
// circular region. The blend modes chosen are standard photo-editing modes:
//   Lip  → Multiply (deepens colour while preserving texture)
//   Eye shadow → Soft-light (gentle tint that feels like pigment)
//   Blush → Screen (brightens with a rosy hue, feels like blush powder)

use crate::skin_mask::point_in_polygon;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MakeupParamsJson {
    pub lip_enabled: bool,
    pub lip_r: u8,
    pub lip_g: u8,
    pub lip_b: u8,
    pub lip_strength: f32,
    pub eye_shadow_enabled: bool,
    pub eye_shadow_r: u8,
    pub eye_shadow_g: u8,
    pub eye_shadow_b: u8,
    pub eye_shadow_strength: f32,
    pub blush_enabled: bool,
    pub blush_r: u8,
    pub blush_g: u8,
    pub blush_b: u8,
    pub blush_strength: f32,
}

// --- Blend helpers ---

#[inline]
fn blend_multiply(base: f32, overlay: f32) -> f32 {
    base * overlay
}

#[inline]
fn blend_screen(base: f32, blend: f32) -> f32 {
    1.0 - (1.0 - base) * (1.0 - blend)
}

/// Apply a solid colour over all pixels inside `poly_px` (pixel-space flat array)
/// using the given blend function and strength.
#[allow(clippy::too_many_arguments)]
fn blend_polygon(
    pixels: &[u8],
    out: &mut [u8],
    width: usize,
    height: usize,
    poly_px: &[f32], // flat pixel-space polygon
    r: f32,
    g: f32,
    b: f32,
    strength: f32,
    blend_fn: fn(f32, f32) -> f32,
) {
    if poly_px.len() < 6 {
        return;
    }
    let n = poly_px.len() / 2;
    let mut xmin = f32::INFINITY;
    let mut xmax = f32::NEG_INFINITY;
    let mut ymin = f32::INFINITY;
    let mut ymax = f32::NEG_INFINITY;
    for i in 0..n {
        let px = poly_px[i * 2];
        let py = poly_px[i * 2 + 1];
        xmin = xmin.min(px);
        xmax = xmax.max(px);
        ymin = ymin.min(py);
        ymax = ymax.max(py);
    }
    let x0 = (xmin.floor() as i32).max(0) as usize;
    let x1 = ((xmax.ceil() as i32 + 1).max(0) as usize).min(width);
    let y0 = (ymin.floor() as i32).max(0) as usize;
    let y1 = ((ymax.ceil() as i32 + 1).max(0) as usize).min(height);

    for py in y0..y1 {
        for px in x0..x1 {
            if !point_in_polygon(px as f32 + 0.5, py as f32 + 0.5, poly_px) {
                continue;
            }
            let i = (py * width + px) * 4;
            let pr = pixels[i] as f32 / 255.0;
            let pg = pixels[i + 1] as f32 / 255.0;
            let pb = pixels[i + 2] as f32 / 255.0;

            let br = blend_fn(pr, r);
            let bg = blend_fn(pg, g);
            let bb = blend_fn(pb, b);

            out[i] = ((pr + (br - pr) * strength) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            out[i + 1] = ((pg + (bg - pg) * strength) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            out[i + 2] = ((pb + (bb - pb) * strength) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Apply a circular Gaussian blush at `(cx, cy)` pixels with given `radius`.
#[allow(clippy::too_many_arguments)]
fn blend_blush_circle(
    pixels: &[u8],
    out: &mut [u8],
    width: usize,
    height: usize,
    cx: f32,
    cy: f32,
    radius: f32,
    r: f32,
    g: f32,
    b: f32,
    strength: f32,
) {
    if radius < 1.0 {
        return;
    }
    let x0 = ((cx - radius).floor() as i32).max(0) as usize;
    let x1 = ((cx + radius).ceil() as i32 + 1).min(width as i32) as usize;
    let y0 = ((cy - radius).floor() as i32).max(0) as usize;
    let y1 = ((cy + radius).ceil() as i32 + 1).min(height as i32) as usize;
    let sigma = radius / 2.5; // Gaussian σ so that falloff reaches ~0 at boundary

    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist2 = dx * dx + dy * dy;
            if dist2 >= radius * radius {
                continue;
            }
            // Gaussian weight
            let gauss = (-dist2 / (2.0 * sigma * sigma)).exp();
            let eff = strength * gauss;

            let i = (py * width + px) * 4;
            let pr = pixels[i] as f32 / 255.0;
            let pg = pixels[i + 1] as f32 / 255.0;
            let pb = pixels[i + 2] as f32 / 255.0;

            let br = blend_screen(pr, r);
            let bg = blend_screen(pg, g);
            let bb = blend_screen(pb, b);

            out[i] = ((pr + (br - pr) * eff) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            out[i + 1] = ((pg + (bg - pg) * eff) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            out[i + 2] = ((pb + (bb - pb) * eff) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Apply makeup effects in-place.
///
/// - `lips_outer`: normalised flat polygon for outer lip boundary
/// - `left_eye`, `right_eye`: normalised eye polygons (used for shadow on upper portion)
/// - `cheeks`: [cx_left, cy_left, cx_right, cy_right] normalised
/// - `params_json`: serialized `MakeupParamsJson`
#[allow(clippy::too_many_arguments)]
pub fn apply_makeup(
    pixels: &[u8],
    width: u32,
    height: u32,
    lips_outer: &[f32],
    left_eye: &[f32],
    right_eye: &[f32],
    cheeks: &[f32],
    params_json: &str,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut out = pixels.to_vec();

    let params: MakeupParamsJson = match serde_json::from_str(params_json) {
        Ok(p) => p,
        Err(_) => return out,
    };

    let denorm = |poly: &[f32]| -> Vec<f32> {
        poly.chunks(2)
            .flat_map(|c| [c[0] * w as f32, c[1] * h as f32])
            .collect()
    };

    // --- Lip colour ---
    if params.lip_enabled && !lips_outer.is_empty() {
        let lips_px = denorm(lips_outer);
        let lr = params.lip_r as f32 / 255.0;
        let lg = params.lip_g as f32 / 255.0;
        let lb = params.lip_b as f32 / 255.0;
        blend_polygon(
            pixels,
            &mut out,
            w,
            h,
            &lips_px,
            lr,
            lg,
            lb,
            params.lip_strength.clamp(0.0, 1.0),
            blend_multiply,
        );
    }

    // --- Eye shadow (upper half of each eye polygon) ---
    if params.eye_shadow_enabled {
        let er = params.eye_shadow_r as f32 / 255.0;
        let eg = params.eye_shadow_g as f32 / 255.0;
        let eb = params.eye_shadow_b as f32 / 255.0;
        let s = params.eye_shadow_strength.clamp(0.0, 1.0);

        for eye in [left_eye, right_eye] {
            if eye.is_empty() {
                continue;
            }
            // Build upper-half polygon: keep only points with y <= eye mid y.
            // Use normal (alpha-composite) blend for vivid プリクラ-style shadow.
            let eye_mid_y: f32 = eye.chunks(2).map(|c| c[1]).sum::<f32>() / (eye.len() / 2) as f32;
            let upper: Vec<f32> = eye
                .chunks(2)
                .filter(|c| c[1] <= eye_mid_y + 0.01)
                .flat_map(|c| [c[0], c[1]])
                .collect();
            if upper.len() >= 6 {
                let upper_px = denorm(&upper);
                blend_polygon(
                    pixels,
                    &mut out,
                    w,
                    h,
                    &upper_px,
                    er,
                    eg,
                    eb,
                    s,
                    |_, overlay| overlay,
                );
            }
        }
    }

    // --- Cheek blush ---
    if params.blush_enabled && cheeks.len() >= 4 {
        let br = params.blush_r as f32 / 255.0;
        let bg = params.blush_g as f32 / 255.0;
        let bb = params.blush_b as f32 / 255.0;
        let s = params.blush_strength.clamp(0.0, 1.0);

        // Blush radius ~ 12% of image width
        let radius = w as f32 * 0.12;

        let cx_left = cheeks[0] * w as f32;
        let cy_left = cheeks[1] * h as f32;
        let cx_right = cheeks[2] * w as f32;
        let cy_right = cheeks[3] * h as f32;

        blend_blush_circle(
            pixels, &mut out, w, h, cx_left, cy_left, radius, br, bg, bb, s,
        );
        blend_blush_circle(
            pixels, &mut out, w, h, cx_right, cy_right, radius, br, bg, bb, s,
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut p = Vec::with_capacity(w * h * 4);
        for _ in 0..(w * h) {
            p.extend_from_slice(&[r, g, b, 255]);
        }
        p
    }

    fn full_quad() -> Vec<f32> {
        vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
    }

    fn no_effect_params() -> String {
        r#"{"lip_enabled":false,"lip_r":200,"lip_g":50,"lip_b":50,"lip_strength":0.5,
            "eye_shadow_enabled":false,"eye_shadow_r":150,"eye_shadow_g":100,"eye_shadow_b":200,"eye_shadow_strength":0.4,
            "blush_enabled":false,"blush_r":255,"blush_g":150,"blush_b":130,"blush_strength":0.3}"#
            .to_string()
    }

    #[test]
    fn all_disabled_returns_input() {
        let p = solid(20, 20, 180, 140, 120);
        let out = apply_makeup(&p, 20, 20, &full_quad(), &[], &[], &[], &no_effect_params());
        assert_eq!(out, p);
    }

    #[test]
    fn lip_effect_changes_pixels() {
        let p = solid(20, 20, 180, 140, 120);
        let params = r#"{"lip_enabled":true,"lip_r":200,"lip_g":20,"lip_b":20,"lip_strength":0.8,
            "eye_shadow_enabled":false,"eye_shadow_r":0,"eye_shadow_g":0,"eye_shadow_b":0,"eye_shadow_strength":0.0,
            "blush_enabled":false,"blush_r":0,"blush_g":0,"blush_b":0,"blush_strength":0.0}"#;
        let out = apply_makeup(&p, 20, 20, &full_quad(), &[], &[], &[], params);
        // Lip multiply: R *= lip_r, so output R should be lower
        let center = (10 * 20 + 10) * 4;
        assert!(
            out[center + 2] < p[center + 2],
            "blue should decrease with red lip multiply"
        );
    }

    #[test]
    fn blush_effect_lightens_cheek() {
        let p = solid(40, 40, 150, 130, 120);
        let params = r#"{"lip_enabled":false,"lip_r":0,"lip_g":0,"lip_b":0,"lip_strength":0.0,
            "eye_shadow_enabled":false,"eye_shadow_r":0,"eye_shadow_g":0,"eye_shadow_b":0,"eye_shadow_strength":0.0,
            "blush_enabled":true,"blush_r":255,"blush_g":150,"blush_b":150,"blush_strength":0.8}"#;
        let cheeks = [0.5f32, 0.5, 0.5, 0.5]; // center cheek
        let out = apply_makeup(&p, 40, 40, &[], &[], &[], &cheeks, params);
        let center = (20 * 40 + 20) * 4;
        // Screen blend with bright color brightens the image
        assert!(out[center] > p[center], "blush screen should brighten R");
    }

    #[test]
    fn alpha_unchanged() {
        let p: Vec<u8> = (0..20 * 20).flat_map(|_| [180u8, 140, 120, 99]).collect();
        let params = r#"{"lip_enabled":true,"lip_r":200,"lip_g":20,"lip_b":20,"lip_strength":0.8,
            "eye_shadow_enabled":false,"eye_shadow_r":0,"eye_shadow_g":0,"eye_shadow_b":0,"eye_shadow_strength":0.0,
            "blush_enabled":false,"blush_r":0,"blush_g":0,"blush_b":0,"blush_strength":0.0}"#;
        let out = apply_makeup(&p, 20, 20, &full_quad(), &[], &[], &[], params);
        for i in (3..out.len()).step_by(4) {
            assert_eq!(out[i], 99, "alpha drift at {i}");
        }
    }

    #[test]
    fn eye_shadow_changes_pixels_when_enabled() {
        let p = solid(40, 40, 150, 150, 150);
        // Oval eye polygon spanning x:[0.1,0.9], y:[0.1,0.5].
        // Upper half (y <= mid ~0.3) forms a valid arch polygon with 7 points.
        #[rustfmt::skip]
        let eye = vec![
            0.1f32, 0.3,  0.2, 0.2,  0.3, 0.15,  0.5, 0.1,
            0.7,    0.15, 0.8, 0.2,  0.9, 0.3,
            0.8,    0.4,  0.7, 0.45, 0.5, 0.5,
            0.3,    0.45, 0.2, 0.4,
        ];
        let params = r#"{"lip_enabled":false,"lip_r":0,"lip_g":0,"lip_b":0,"lip_strength":0.0,
            "eye_shadow_enabled":true,"eye_shadow_r":200,"eye_shadow_g":100,"eye_shadow_b":230,"eye_shadow_strength":0.8,
            "blush_enabled":false,"blush_r":0,"blush_g":0,"blush_b":0,"blush_strength":0.0}"#;
        let out = apply_makeup(&p, 40, 40, &[], &eye, &[], &[], params);
        // Probe pixel inside the upper-half arch: approx (0.5, 0.2) normalised → pixel (20, 8).
        let probe = (8 * 40 + 20) * 4;
        let changed = (out[probe] != p[probe])
            || (out[probe + 1] != p[probe + 1])
            || (out[probe + 2] != p[probe + 2]);
        assert!(
            changed,
            "eye shadow should change pixels inside the upper eye polygon"
        );
    }

    #[test]
    fn pixels_outside_lip_polygon_unchanged() {
        let p = solid(40, 40, 180, 140, 120);
        // Lip polygon covers only the upper-left 25% of the image.
        let lip = vec![0.0f32, 0.0, 0.5, 0.0, 0.5, 0.5, 0.0, 0.5];
        let params = r#"{"lip_enabled":true,"lip_r":255,"lip_g":0,"lip_b":0,"lip_strength":1.0,
            "eye_shadow_enabled":false,"eye_shadow_r":0,"eye_shadow_g":0,"eye_shadow_b":0,"eye_shadow_strength":0.0,
            "blush_enabled":false,"blush_r":0,"blush_g":0,"blush_b":0,"blush_strength":0.0}"#;
        let out = apply_makeup(&p, 40, 40, &lip, &[], &[], &[], params);
        // Bottom-right corner pixel is outside the polygon → unchanged.
        let corner = (39 * 40 + 39) * 4;
        assert_eq!(out[corner], p[corner], "outside pixel R");
        assert_eq!(out[corner + 1], p[corner + 1], "outside pixel G");
        assert_eq!(out[corner + 2], p[corner + 2], "outside pixel B");
    }
}
