// Automatic blemish removal via Normalized Convolution.
//
// Detection (gated by skin_mask):
//   * DoG (Difference of Gaussians) on luminance — Lindeberg 1998 blob theory.
//     Catches small dark spots: moles, freckles, dark pores, the eyebrow ends
//     that bled past the exclusion polygon, etc.
//   * Redness score r = (R+1) / (G+B+1) plus an absolute (R-G) margin —
//     catches pimples and broken capillaries that aren't darker than skin but
//     are colour-shifted toward red.
//
// Repair (the rewritten part):
//   We previously inpainted by blending toward a global σ=10 Gaussian of the
//   whole image. At 640×480 with the face occupying ~⅓ of the frame, that
//   "reference" is contaminated by hair, eyebrows, lips and background — so
//   the patched colour drifts and texture flattens.
//
//   The new repair uses **Normalized Convolution** (Knutsson & Westin 1993):
//
//       inpainted(p) = Σ_q [ G(p-q) · c(q) · I(q) ]
//                      ─────────────────────────────
//                      Σ_q [ G(p-q) · c(q) ]
//
//   where c(q) = (1 − blemish_score) · skin_mask. Pixels confidently identified
//   as "normal skin" vote on the patched colour; the detected blemish votes
//   with weight zero. Implemented with separable box blur (radius scales with
//   image size) so it stays O(N) and fast in wasm.
//
//   The result: each blemish pixel is replaced by the local mean of the
//   *surrounding healthy skin*. Texture and tone of the neighbourhood survive,
//   only the spot disappears.

const REPAIR_RADIUS_FRAC: f32 = 0.012; // ≈ 8 px on 640×480 → ~16 px diameter, larger than typical spots

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
                let nx = (x as i64 + ki as i64 - radius as i64).clamp(0, width as i64 - 1) as usize;
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
                let ny =
                    (y as i64 + ki as i64 - radius as i64).clamp(0, height as i64 - 1) as usize;
                s += tmp[ny * width + x] * kw;
            }
            dst[y * width + x] = s;
        }
    }
    dst
}

/// Separable box blur on a f32 channel with edge-replicating boundaries.
fn box_blur_f32(src: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    let n = width * height;
    if radius == 0 {
        return src.to_vec();
    }
    let win = (radius * 2 + 1) as f32;
    let mut tmp = vec![0f32; n];
    for y in 0..height {
        let row = y * width;
        let mut acc = (radius as f32) * src[row];
        for k in 0..=radius.min(width - 1) {
            acc += src[row + k];
        }
        for x in 0..width {
            tmp[row + x] = acc / win;
            let left = x.saturating_sub(radius);
            let right = (x + radius + 1).min(width - 1);
            acc += src[row + right] - src[row + left];
        }
    }
    let mut dst = vec![0f32; n];
    for x in 0..width {
        let mut acc = (radius as f32) * tmp[x];
        for k in 0..=radius.min(height - 1) {
            acc += tmp[k * width + x];
        }
        for y in 0..height {
            dst[y * width + x] = acc / win;
            let top = y.saturating_sub(radius);
            let bot = (y + radius + 1).min(height - 1);
            acc += tmp[bot * width + x] - tmp[top * width + x];
        }
    }
    dst
}

#[inline]
fn luminance_f32(r: u8, g: u8, b: u8) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

/// Detect blemishes and inpaint via normalized convolution.
///
/// `skin_mask` (0..255) gates detection AND defines the trusted neighbourhood
/// for inpainting — non-skin pixels never contribute to the patched colour.
/// `strength` (0..1) scales the final blend at full detection.
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
    let strength = strength.clamp(0.0, 1.0);

    // 1) Luminance for blob detection.
    let mut lum = vec![0f32; n];
    for k in 0..n {
        lum[k] = luminance_f32(pixels[k * 4], pixels[k * 4 + 1], pixels[k * 4 + 2]);
    }

    // 2) DoG dark-blob score. σ_small captures pore/freckle scale, σ_large the
    //    surrounding skin tone. Score saturates at DARK_NORM above the threshold.
    let g_small = gaussian_blur_f32(&lum, width, height, 1.2);
    let g_large = gaussian_blur_f32(&lum, width, height, 5.0);
    let dark_thresh = 6.0f32;
    let dark_norm = 12.0f32;
    let mut dark_score = vec![0f32; n];
    for k in 0..n {
        let d = (g_large[k] - g_small[k]).max(0.0);
        dark_score[k] = ((d - dark_thresh) / dark_norm).clamp(0.0, 1.0);
    }

    // 3) Redness score: ratio AND absolute (R-G) margin together — ratio alone
    //    flags any warm pixel; the margin requirement keeps it specific to red
    //    inflammation rather than baseline skin warmth.
    let red_ratio_thresh = 1.45f32;
    let red_ratio_norm = 0.35f32;
    let red_margin_thresh = 35.0f32;
    let red_margin_norm = 25.0f32;
    let mut red_score = vec![0f32; n];
    for k in 0..n {
        let r = pixels[k * 4] as f32;
        let g = pixels[k * 4 + 1] as f32;
        let b = pixels[k * 4 + 2] as f32;
        let ratio = (r + 1.0) / (g + b + 1.0);
        let ratio_score = ((ratio - red_ratio_thresh) / red_ratio_norm).clamp(0.0, 1.0);
        let margin = r - g;
        let margin_score = ((margin - red_margin_thresh) / red_margin_norm).clamp(0.0, 1.0);
        red_score[k] = ratio_score.min(margin_score);
    }

    // 4) Combine and gate by skin mask.
    let mut score = vec![0f32; n];
    for k in 0..n {
        let m = skin_mask[k] as f32 / 255.0;
        score[k] = dark_score[k].max(red_score[k]) * m;
    }

    // 5) Normalized convolution. confidence = (1 − score) · skin_mask gives the
    //    weight of "trusted normal-skin" testimony. Sum of weighted RGB ÷ sum
    //    of weights = the locally-supported skin colour at this pixel.
    let mut confidence = vec![0f32; n];
    for k in 0..n {
        let m = skin_mask[k] as f32 / 255.0;
        confidence[k] = (1.0 - score[k]) * m;
    }
    let radius = ((width.max(height) as f32) * REPAIR_RADIUS_FRAC).round() as usize;
    let radius = radius.max(3);

    let blurred_conf = box_blur_f32(&confidence, width, height, radius);
    let mut patched = [vec![0f32; n], vec![0f32; n], vec![0f32; n]];
    for c in 0..3 {
        let mut weighted = vec![0f32; n];
        for k in 0..n {
            weighted[k] = pixels[k * 4 + c] as f32 * confidence[k];
        }
        let blurred_w = box_blur_f32(&weighted, width, height, radius);
        for k in 0..n {
            // Avoid div-by-zero where the window is entirely outside the skin
            // mask — fall back to the original pixel (the score will be 0
            // there anyway, so the blend is a no-op).
            let denom = blurred_conf[k];
            patched[c][k] = if denom > 1e-4 {
                blurred_w[k] / denom
            } else {
                pixels[k * 4 + c] as f32
            };
        }
    }

    // 6) Composite. The inpainted colour replaces the original in proportion
    //    to the blemish score (× user strength). Skin texture outside the
    //    spot is untouched.
    let mut out = pixels.to_vec();
    for k in 0..n {
        let t = score[k] * strength;
        for c in 0..3 {
            let a = pixels[k * 4 + c] as f32;
            let b = patched[c][k];
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
        let w = 81;
        let h = 81;
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
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 1.0);
        let center = (cy * w + cx) * 4;
        // After inpainting, the center should be close to skin tone (220) — not
        // the original 80, and not far below skin tone either (no dark drift).
        assert!(out[center] > 195, "dark spot still dark: R={}", out[center]);
    }

    #[test]
    fn red_pimple_neutralised() {
        let w = 81;
        let h = 81;
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
        let out = remove_blemish(&pixels, w, h, &mask, 1.0);
        let center = (cy * w + cx) * 4;
        // Red drops a lot, green/blue lift toward skin baseline.
        assert!(out[center] < 240, "red still saturated: R={}", out[center]);
        assert!(
            out[center + 1] > 120,
            "green not lifted: G={}",
            out[center + 1]
        );
    }

    #[test]
    fn mask_zero_blocks_inpaint() {
        let w = 81;
        let h = 81;
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

    #[test]
    fn distant_pixels_untouched_by_local_repair() {
        // Single dark spot at center; verify pixels far outside the repair
        // window are bit-identical to the input. This is what normalized
        // convolution buys us over the previous global-Gaussian reference.
        let w = 161;
        let h = 161;
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
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 1.0);
        // Pixel near the corner — well outside the repair radius (~ max(w,h)*0.012 ≈ 2 px,
        // clamped up to 3, plus DoG support ~15 px). 50 px away is comfortably outside.
        let far = (10 * w + 10) * 4;
        for c in 0..3 {
            assert_eq!(
                out[far + c],
                pixels[far + c],
                "far pixel changed at c={}",
                c
            );
        }
    }

    #[test]
    fn flat_warm_skin_does_not_trigger_redness() {
        // Pure warm skin (no inflammation) — must NOT be detected as red blemish.
        // Tests that the absolute (R-G) margin requirement keeps baseline skin safe.
        let w = 30;
        let h = 30;
        let pixels: Vec<u8> = (0..w * h).flat_map(|_| [230u8, 190, 160, 255]).collect();
        let mask = full_mask(w * h);
        let out = remove_blemish(&pixels, w, h, &mask, 1.0);
        for k in 0..(w * h) {
            for c in 0..3 {
                let d = (out[k * 4 + c] as i32 - pixels[k * 4 + c] as i32).abs();
                assert!(d <= 3, "warm skin drifted {} at {}/{}", d, k, c);
            }
        }
    }
}
