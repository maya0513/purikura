// Automatic blemish removal.
//
// Two complementary detectors fire on the skin mask:
//   * DoG (Difference of Gaussians) on luminance — Lindeberg 1998 blob theory.
//     Captures small dark spots (moles, freckles, dark pores).
//   * Redness score r = (R+1) / (G+B+1) — captures pimples and broken
//     capillaries that aren't darker than skin but are colour-shifted.
//
// Detected pixels are inpainted by blending toward a low-frequency reference
// (large-σ Gaussian of the input), which is essentially the "spot heal" trick
// from Lipowezky & Cahen, "Automatic freckles detection and retouching".

/// 1-D Gaussian kernel, normalized.
fn gaussian_kernel_1d(sigma: f32) -> Vec<f32> {
    let radius = (3.0 * sigma).ceil().max(1.0) as usize;
    let mut k = Vec::with_capacity(radius * 2 + 1);
    for i in 0..=(radius * 2) {
        let d = i as f32 - radius as f32;
        k.push((-0.5 * d * d / (sigma * sigma)).exp());
    }
    let s: f32 = k.iter().sum();
    for v in &mut k {
        *v /= s;
    }
    k
}

/// Separable Gaussian blur on a single f32 channel. Replicates boundaries.
fn gaussian_blur_f32(src: &[f32], width: usize, height: usize, sigma: f32) -> Vec<f32> {
    let kernel = gaussian_kernel_1d(sigma);
    let radius = kernel.len() / 2;
    let n = width * height;
    let mut tmp = vec![0f32; n];
    for y in 0..height {
        for x in 0..width {
            let mut s = 0f32;
            for (ki, &kw) in kernel.iter().enumerate() {
                let nx = (x as i64 + ki as i64 - radius as i64)
                    .clamp(0, width as i64 - 1) as usize;
                s += src[y * width + nx] * kw;
            }
            tmp[y * width + x] = s;
        }
    }
    let mut dst = vec![0f32; n];
    for y in 0..height {
        for x in 0..width {
            let mut s = 0f32;
            for (ki, &kw) in kernel.iter().enumerate() {
                let ny = (y as i64 + ki as i64 - radius as i64)
                    .clamp(0, height as i64 - 1) as usize;
                s += tmp[ny * width + x] * kw;
            }
            dst[y * width + x] = s;
        }
    }
    dst
}

/// Gaussian blur on RGBA. Alpha pass-through.
fn gaussian_blur_rgba(pixels: &[u8], width: usize, height: usize, sigma: f32) -> Vec<u8> {
    let n = width * height;
    let mut chans: [Vec<f32>; 3] = [vec![0f32; n], vec![0f32; n], vec![0f32; n]];
    for k in 0..n {
        for c in 0..3 {
            chans[c][k] = pixels[k * 4 + c] as f32;
        }
    }
    for c in 0..3 {
        chans[c] = gaussian_blur_f32(&chans[c], width, height, sigma);
    }
    let mut out = vec![0u8; pixels.len()];
    for k in 0..n {
        for c in 0..3 {
            out[k * 4 + c] = chans[c][k].clamp(0.0, 255.0) as u8;
        }
        out[k * 4 + 3] = pixels[k * 4 + 3];
    }
    out
}

#[inline]
fn luminance_f32(r: u8, g: u8, b: u8) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

/// One round of binary 4-neighbour dilation.
fn dilate_once(mask: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut out = mask.to_vec();
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let mut m = mask[i];
            if x > 0 { m = m.max(mask[i - 1]); }
            if x + 1 < width { m = m.max(mask[i + 1]); }
            if y > 0 { m = m.max(mask[i - width]); }
            if y + 1 < height { m = m.max(mask[i + width]); }
            out[i] = m;
        }
    }
    out
}

/// Detect and inpaint blemishes inside the skin region.
///
/// `skin_mask` (0..255) gates detection — pixels with mask=0 are never inpainted.
/// `strength` (0..1) scales the inpaint blend at full detection (typical 0.95).
pub fn remove_blemish(
    pixels: &[u8],
    width: usize,
    height: usize,
    skin_mask: &[u8],
    strength: f32,
) -> Vec<u8> {
    let n = width * height;
    debug_assert_eq!(pixels.len(), n * 4);
    debug_assert_eq!(skin_mask.len(), n);

    // 1) Luminance.
    let mut lum = vec![0f32; n];
    for k in 0..n {
        lum[k] = luminance_f32(pixels[k * 4], pixels[k * 4 + 1], pixels[k * 4 + 2]);
    }

    // 2) DoG: small Gaussian minus larger Gaussian.
    let g1 = gaussian_blur_f32(&lum, width, height, 1.5);
    let g2 = gaussian_blur_f32(&lum, width, height, 4.0);
    // Dark blob ⇒ DoG = g1 - g2 < 0  (smaller scale dips below larger scale).
    // We'll express dark_score = max(0, g2 - g1) / norm.
    // Threshold tuned for [0..255] luminance: 4.0 catches subtle freckles too.
    let dark_thresh: f32 = 4.0;
    let mut dark_score = vec![0f32; n];
    for k in 0..n {
        let d = (g2[k] - g1[k]).max(0.0);
        dark_score[k] = ((d - dark_thresh) / dark_thresh).clamp(0.0, 1.0);
    }

    // 3) Redness score (R+1)/(G+B+1). Skin baseline ~ 1.0, pimples > 1.4.
    let red_thresh: f32 = 1.35;
    let red_norm: f32 = 0.4; // saturate at red_thresh + red_norm
    let mut red_score = vec![0f32; n];
    for k in 0..n {
        let r = pixels[k * 4] as f32 + 1.0;
        let gb = pixels[k * 4 + 1] as f32 + pixels[k * 4 + 2] as f32 + 1.0;
        let ratio = r / gb;
        red_score[k] = ((ratio - red_thresh) / red_norm).clamp(0.0, 1.0);
    }

    // 4) Combine, gate by skin mask.
    let mut score = vec![0f32; n];
    for k in 0..n {
        let m = skin_mask[k] as f32 / 255.0;
        score[k] = dark_score[k].max(red_score[k]) * m;
    }

    // 5) Slight dilation so we catch the blemish boundary, not just the centre.
    let score = dilate_once(&score, width, height);

    // 6) Inpaint reference: large-σ blur of the source RGBA.
    //    σ=10 ≈ 60-pixel kernel — well beyond typical pore/spot diameter, so
    //    the blurred image is essentially "skin tone with hair/lips smeared in"
    //    — but we only blend where score>0 (and score is gated by skin_mask).
    let low_freq = gaussian_blur_rgba(pixels, width, height, 10.0);

    // 7) Composite.
    let mut out = pixels.to_vec();
    for k in 0..n {
        let t = score[k] * strength;
        for c in 0..3 {
            let a = pixels[k * 4 + c] as f32;
            let b = low_freq[k * 4 + c] as f32;
            out[k * 4 + c] = (a + (b - a) * t).clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skin_image(w: usize, h: usize) -> Vec<u8> {
        // Uniform skin tone (R=220, G=180, B=150).
        (0..w * h).flat_map(|_| [220u8, 180, 150, 255]).collect()
    }

    fn full_mask(n: usize) -> Vec<u8> {
        vec![255u8; n]
    }

    #[test]
    fn flat_skin_unchanged() {
        let w = 30;
        let h = 30;
        let pixels = make_skin_image(w, h);
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 0.95);
        for k in 0..(w * h) {
            for c in 0..3 {
                let d = (out[k * 4 + c] as i32 - pixels[k * 4 + c] as i32).abs();
                assert!(d <= 2, "flat skin drift {} at {}/{}", d, k, c);
            }
        }
    }

    #[test]
    fn dark_spot_lifted_toward_skin_tone() {
        // Skin background with a single dark spot at center.
        let w = 41;
        let h = 41;
        let mut pixels = make_skin_image(w, h);
        let cx = w / 2;
        let cy = h / 2;
        // Make a small dark circle (radius 2)
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx * dx + dy * dy <= 4 {
                    let x = (cx as i32 + dx) as usize;
                    let y = (cy as i32 + dy) as usize;
                    let i = (y * w + x) * 4;
                    pixels[i] = 80;
                    pixels[i + 1] = 60;
                    pixels[i + 2] = 50;
                }
            }
        }
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 0.95);
        let center = (cy * w + cx) * 4;
        // After inpainting, the center should be much closer to skin tone than the original 80.
        assert!(
            out[center] > 160,
            "dark spot still dark: R={}",
            out[center]
        );
    }

    #[test]
    fn red_pimple_neutralised() {
        // Skin background with a single very-red spot at center.
        let w = 41;
        let h = 41;
        let mut pixels = make_skin_image(w, h);
        let cx = w / 2;
        let cy = h / 2;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx * dx + dy * dy <= 4 {
                    let x = (cx as i32 + dx) as usize;
                    let y = (cy as i32 + dy) as usize;
                    let i = (y * w + x) * 4;
                    pixels[i] = 250;
                    pixels[i + 1] = 60;
                    pixels[i + 2] = 60;
                }
            }
        }
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 0.95);
        let center = (cy * w + cx) * 4;
        // Red drops a lot, green/blue rise toward skin baseline.
        assert!(out[center] < 240, "red still saturated: R={}", out[center]);
        assert!(
            out[center + 1] > 120,
            "green not lifted: G={}",
            out[center + 1]
        );
    }

    #[test]
    fn mask_zero_blocks_inpaint() {
        let w = 41;
        let h = 41;
        let mut pixels = make_skin_image(w, h);
        let cx = w / 2;
        let cy = h / 2;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx * dx + dy * dy <= 4 {
                    let x = (cx as i32 + dx) as usize;
                    let y = (cy as i32 + dy) as usize;
                    let i = (y * w + x) * 4;
                    pixels[i] = 80;
                    pixels[i + 1] = 60;
                    pixels[i + 2] = 50;
                }
            }
        }
        let mask = vec![0u8; w * h]; // mask blocks everything
        let out = remove_blemish(&pixels, w, h, &mask, 0.95);
        let center = (cy * w + cx) * 4;
        // No change because mask=0
        assert_eq!(out[center], 80);
    }

    #[test]
    fn alpha_preserved() {
        let w = 20;
        let h = 20;
        let pixels: Vec<u8> = (0..w * h).flat_map(|_| [220u8, 180, 150, 137]).collect();
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 0.95);
        for k in 0..(w * h) {
            assert_eq!(out[k * 4 + 3], 137);
        }
    }
}
