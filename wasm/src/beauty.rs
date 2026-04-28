// Skin beautification orchestrator.
//
// Pipeline (gated by `skin_mask`):
//   1. Frequency separation via Guided Filter (He et al. ECCV 2010): low =
//      smooth tone, high = texture/pores. Standard professional retouching.
//   2. Recombine with low + α·high where α = TEXTURE_KEEP (avoids the
//      "plastic skin" failure mode of pure low-freq).
//   3. Pull skin colour toward the masked mean → evens out tone unevenness.
//   4. Brighten toward white (プリクラ standard).
//   5. Desaturate slightly (white skin look).
//   6. Composite onto the original using mask × strength.

use crate::guided_filter::guided_filter_rgba;

const TEXTURE_KEEP: f32 = 0.25;
const PULL_TO_MEAN: f32 = 0.20;
const BRIGHTEN: f32 = 0.18;
const DESATURATE: f32 = 0.12;

const GUIDED_RADIUS: usize = 12;
const GUIDED_EPS: f32 = 0.002;

/// Apply beauty pipeline. `skin_mask` (u8 0..255) gates every effect.
/// `strength` ∈ [0..1] scales the final blend.
pub fn apply_beauty(
    pixels: &[u8],
    width: usize,
    height: usize,
    skin_mask: &[u8],
    strength: f32,
) -> Vec<u8> {
    let n = width * height;
    debug_assert_eq!(pixels.len(), n * 4);
    debug_assert_eq!(skin_mask.len(), n);
    let strength = strength.clamp(0.0, 1.0);

    // 1+2. Frequency separation, recombine with reduced texture.
    let low = guided_filter_rgba(pixels, width, height, GUIDED_RADIUS, GUIDED_EPS);
    let mut smoothed = vec![0u8; pixels.len()];
    for k in 0..n {
        for c in 0..3 {
            let orig = pixels[k * 4 + c] as f32;
            let l = low[k * 4 + c] as f32;
            let high = orig - l;
            let v = l + TEXTURE_KEEP * high;
            smoothed[k * 4 + c] = v.clamp(0.0, 255.0) as u8;
        }
        smoothed[k * 4 + 3] = pixels[k * 4 + 3];
    }

    // 3. Mean skin tone (weighted by mask).
    let (mean_r, mean_g, mean_b) = skin_mean(&smoothed, skin_mask);

    // 4-6. Apply per-pixel adjustments composited onto original via mask*strength.
    let mut out = pixels.to_vec();
    for k in 0..n {
        let m = (skin_mask[k] as f32 / 255.0) * strength;
        if m < 0.001 {
            continue;
        }
        let mut r = smoothed[k * 4] as f32;
        let mut g = smoothed[k * 4 + 1] as f32;
        let mut b = smoothed[k * 4 + 2] as f32;

        // Tone uniformity.
        r += (mean_r - r) * PULL_TO_MEAN;
        g += (mean_g - g) * PULL_TO_MEAN;
        b += (mean_b - b) * PULL_TO_MEAN;

        // Brightening.
        r += BRIGHTEN * (255.0 - r);
        g += BRIGHTEN * (255.0 - g);
        b += BRIGHTEN * (255.0 - b);

        // Desaturation toward luminance.
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        r += (lum - r) * DESATURATE;
        g += (lum - g) * DESATURATE;
        b += (lum - b) * DESATURATE;

        let or_ = pixels[k * 4] as f32;
        let og = pixels[k * 4 + 1] as f32;
        let ob = pixels[k * 4 + 2] as f32;
        out[k * 4] = (or_ + (r - or_) * m).clamp(0.0, 255.0) as u8;
        out[k * 4 + 1] = (og + (g - og) * m).clamp(0.0, 255.0) as u8;
        out[k * 4 + 2] = (ob + (b - ob) * m).clamp(0.0, 255.0) as u8;
    }
    out
}

fn skin_mean(pixels: &[u8], mask: &[u8]) -> (f32, f32, f32) {
    let mut sr = 0f64;
    let mut sg = 0f64;
    let mut sb = 0f64;
    let mut sw = 0f64;
    for k in 0..mask.len() {
        let w = mask[k] as f64;
        sr += pixels[k * 4] as f64 * w;
        sg += pixels[k * 4 + 1] as f64 * w;
        sb += pixels[k * 4 + 2] as f64 * w;
        sw += w;
    }
    if sw < 1.0 {
        return (220.0, 180.0, 150.0);
    }
    (
        (sr / sw) as f32,
        (sg / sw) as f32,
        (sb / sw) as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin_pixels(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        (0..w * h).flat_map(|_| [r, g, b, 255]).collect()
    }

    #[test]
    fn mask_zero_returns_input() {
        let pixels = skin_pixels(20, 20, 200, 160, 130);
        let mask = vec![0u8; 20 * 20];
        let out = apply_beauty(&pixels, 20, 20, &mask, 1.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn strength_zero_returns_input() {
        let pixels = skin_pixels(20, 20, 200, 160, 130);
        let mask = vec![255u8; 20 * 20];
        let out = apply_beauty(&pixels, 20, 20, &mask, 0.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn alpha_preserved() {
        let pixels: Vec<u8> = (0..20 * 20).flat_map(|_| [200u8, 160, 130, 137]).collect();
        let mask = vec![255u8; 20 * 20];
        let out = apply_beauty(&pixels, 20, 20, &mask, 1.0);
        for k in 0..(20 * 20) {
            assert_eq!(out[k * 4 + 3], 137);
        }
    }

    #[test]
    fn full_strength_brightens_skin() {
        let pixels = skin_pixels(20, 20, 200, 160, 130);
        let mask = vec![255u8; 20 * 20];
        let out = apply_beauty(&pixels, 20, 20, &mask, 1.0);
        // R rises (brightening dominates over the small desaturate pull-down on
        // a warm tone where lum < R).
        assert!(out[0] > pixels[0], "R didn't rise: {} vs {}", out[0], pixels[0]);
        // G rises too.
        assert!(out[1] > pixels[1], "G didn't rise: {} vs {}", out[1], pixels[1]);
        // Alpha unchanged
        assert_eq!(out[3], 255);
    }

    #[test]
    fn skin_mean_excludes_zero_mask() {
        let mut pixels = skin_pixels(10, 10, 100, 100, 100);
        // Add bright pixels in masked-out region — should NOT raise the mean.
        for i in 0..30 {
            pixels[i * 4] = 250;
            pixels[i * 4 + 1] = 250;
            pixels[i * 4 + 2] = 250;
        }
        let mut mask = vec![255u8; 10 * 10];
        for i in 0..30 {
            mask[i] = 0;
        }
        let (r, _g, _b) = skin_mean(&pixels, &mask);
        assert!((r - 100.0).abs() < 1.0, "mean drifted to {}", r);
    }
}
