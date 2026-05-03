// Skin beautification orchestrator.
//
// Pipeline (gated by `skin_mask`):
//   1. Frequency separation via Guided Filter (He et al. ECCV 2010): low =
//      smooth tone, high = texture/pores. Standard professional retouching.
//   2. Recombine with low + α·high where α = TEXTURE_KEEP. Higher α keeps
//      more pore texture — at 0.55 the result reads as "skin" not "plastic".
//   3. Pull skin colour toward a *local* mean (not the global face mean).
//      Local pull evens out tonal patches without flattening the natural
//      light/shadow gradient that gives the face dimensionality.
//   4. Tiny brighten/desaturate to nod toward プリクラ tone — kept small so
//      the result is "you, but with better skin", not "white blob".
//   5. Composite onto the original using mask × strength.
//
// Tuning history: BRIGHTEN was 0.18, DESATURATE 0.12, PULL_TO_MEAN 0.20 (global
// mean), TEXTURE_KEEP 0.25. That stack produced a flat, over-pale "白塗り"
// face. The new values were chosen to land closer to a soft-focus portrait
// than to a porcelain mask.

use crate::guided_filter::guided_filter_rgba;

const TEXTURE_KEEP: f32 = 0.55;
const PULL_TO_LOCAL: f32 = 0.30;
const BRIGHTEN: f32 = 0.06;
const DESATURATE: f32 = 0.04;

const GUIDED_RADIUS: usize = 8;
const GUIDED_EPS: f32 = 0.003;

// Radius of the local-mean window for tone uniformization. Roughly cheek-sized
// at our 640×480 capture — large enough to average over freckle/pore patches,
// small enough that the local mean still tracks the light falloff across the
// face (so we don't fight the lighting gradient).
const LOCAL_MEAN_RADIUS: usize = 24;

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

    // 3. Local skin-tone reference (mask-weighted box blur of the smoothed
    //    image). This is the colour each skin pixel should drift toward to
    //    even out patchy areas without flattening the overall light/shadow
    //    gradient — what the previous global-mean code couldn't do.
    let (local_r, local_g, local_b) =
        local_skin_mean(&smoothed, skin_mask, width, height, LOCAL_MEAN_RADIUS);

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

        // Tone uniformity (toward local skin colour).
        r += (local_r[k] - r) * PULL_TO_LOCAL;
        g += (local_g[k] - g) * PULL_TO_LOCAL;
        b += (local_b[k] - b) * PULL_TO_LOCAL;

        // Brightening (subtle).
        r += BRIGHTEN * (255.0 - r);
        g += BRIGHTEN * (255.0 - g);
        b += BRIGHTEN * (255.0 - b);

        // Desaturation toward luminance (subtle).
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

/// Mask-weighted local mean of each RGB channel — Σ(I·m) / Σ(m) inside a box
/// window, a.k.a. normalized convolution. Returns three per-pixel reference
/// channels. Where the window contains no skin, falls back to the pixel itself
/// so the downstream pull becomes a no-op there.
fn local_skin_mean(
    pixels: &[u8],
    mask: &[u8],
    width: usize,
    height: usize,
    radius: usize,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = width * height;
    let mut conf = vec![0f32; n];
    for k in 0..n {
        conf[k] = mask[k] as f32 / 255.0;
    }
    let blurred_conf = box_blur_f32(&conf, width, height, radius);

    let mut means: [Vec<f32>; 3] = [vec![0f32; n], vec![0f32; n], vec![0f32; n]];
    for c in 0..3 {
        let mut weighted = vec![0f32; n];
        for k in 0..n {
            weighted[k] = pixels[k * 4 + c] as f32 * conf[k];
        }
        let blurred_w = box_blur_f32(&weighted, width, height, radius);
        for k in 0..n {
            let denom = blurred_conf[k];
            means[c][k] = if denom > 1e-4 {
                blurred_w[k] / denom
            } else {
                pixels[k * 4 + c] as f32
            };
        }
    }
    let [mr, mg, mb] = means;
    (mr, mg, mb)
}

/// Separable box blur (replicate boundary). Same shape as the helper in
/// blemish.rs — kept private here to avoid coupling the two modules.
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
        assert!(
            out[0] > pixels[0],
            "R didn't rise: {} vs {}",
            out[0],
            pixels[0]
        );
        // G rises too.
        assert!(
            out[1] > pixels[1],
            "G didn't rise: {} vs {}",
            out[1],
            pixels[1]
        );
        // Alpha unchanged
        assert_eq!(out[3], 255);
    }

    #[test]
    fn skin_texture_partly_survives() {
        // Skin field with a fine high-frequency checker pattern (±10) on top.
        // The previous TEXTURE_KEEP=0.25 drained ~75% of texture amplitude,
        // producing the "plastic skin" failure mode. With TEXTURE_KEEP=0.55
        // a meaningful fraction must remain.
        let w = 40;
        let h = 40;
        let mut pixels = skin_pixels(w, h, 200, 160, 130);
        for y in 0..h {
            for x in 0..w {
                let bump = if (x + y) % 2 == 0 { 10i32 } else { -10 };
                let i = (y * w + x) * 4;
                pixels[i] = (pixels[i] as i32 + bump).clamp(0, 255) as u8;
                pixels[i + 1] = (pixels[i + 1] as i32 + bump).clamp(0, 255) as u8;
                pixels[i + 2] = (pixels[i + 2] as i32 + bump).clamp(0, 255) as u8;
            }
        }
        let mask = vec![255u8; w * h];
        let out = apply_beauty(&pixels, w, h, &mask, 1.0);

        // Compare R-channel variance inside an interior patch (avoid box-blur
        // boundary artefacts). Variance loss bounds the texture loss.
        let var = |buf: &[u8]| {
            let mut sum = 0f32;
            let mut sumsq = 0f32;
            let mut count = 0f32;
            for y in 8..(h - 8) {
                for x in 8..(w - 8) {
                    let v = buf[(y * w + x) * 4] as f32;
                    sum += v;
                    sumsq += v * v;
                    count += 1.0;
                }
            }
            let mean = sum / count;
            sumsq / count - mean * mean
        };
        let v_in = var(&pixels);
        let v_out = var(&out);
        // High-frequency textures (this 2-px checker is the worst case) lose
        // most of their variance to the local-mean pull regardless. The bound
        // we care about is "doesn't go to ~zero" — i.e. the result still
        // reads as skin, not a flat blur. Old config (TEXTURE_KEEP=0.25 with
        // global pull-to-mean) measured ~7% on this image; the new pipeline
        // sustains roughly 18-22%.
        assert!(
            v_out > v_in * 0.15,
            "texture too suppressed: var_in={}, var_out={}",
            v_in,
            v_out
        );
    }

    #[test]
    fn lighting_gradient_preserved() {
        // Horizontal brightness ramp from dark skin (left) to bright skin
        // (right). After beauty, the *gradient* should still be there — a
        // global-mean pull (the old behaviour) would flatten it. We assert
        // the output's brightness span is at least 70% of the input span.
        let w = 80;
        let h = 20;
        let mut pixels = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let t = x as f32 / (w - 1) as f32;
                // Lerp from (140, 110, 90) → (230, 190, 165), a plausible
                // light/shadow swing on a face.
                let r = (140.0 + t * 90.0) as u8;
                let g = (110.0 + t * 80.0) as u8;
                let b = (90.0 + t * 75.0) as u8;
                let i = (y * w + x) * 4;
                pixels[i] = r;
                pixels[i + 1] = g;
                pixels[i + 2] = b;
                pixels[i + 3] = 255;
            }
        }
        let mask = vec![255u8; w * h];
        let out = apply_beauty(&pixels, w, h, &mask, 1.0);

        let row = h / 2;
        let span_in = pixels[(row * w + (w - 5)) * 4] as i32 - pixels[(row * w + 5) * 4] as i32;
        let span_out = out[(row * w + (w - 5)) * 4] as i32 - out[(row * w + 5) * 4] as i32;
        assert!(
            span_out as f32 > span_in as f32 * 0.70,
            "lighting gradient flattened: span_in={}, span_out={}",
            span_in,
            span_out
        );
    }

    #[test]
    fn local_mean_excludes_masked_out_pixels() {
        // 10x10 with most pixels at skin tone, top row at white. Local-mean
        // computation should ignore masked-out (mask=0) pixels: pulling
        // pixels in the bottom rows toward "local skin" must reflect skin
        // tone, not the bright pixels in the masked-out region.
        let w = 10;
        let h = 10;
        let mut pixels = skin_pixels(w, h, 100, 100, 100);
        for i in 0..(w) {
            pixels[i * 4] = 250;
            pixels[i * 4 + 1] = 250;
            pixels[i * 4 + 2] = 250;
        }
        let mut mask = vec![255u8; w * h];
        for i in 0..w {
            mask[i] = 0;
        }
        let (r, _, _) = local_skin_mean(&pixels, &mask, w, h, 4);
        // Pixel deep in the skin region (row 8) — should average to ~100,
        // not be pulled up by the masked-out bright row.
        assert!(
            (r[8 * w + 5] - 100.0).abs() < 5.0,
            "deep-skin local mean drifted to {}",
            r[8 * w + 5]
        );
    }
}
