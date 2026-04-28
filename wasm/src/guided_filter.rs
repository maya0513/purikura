// Guided Image Filter (He, Sun, Tang. ECCV 2010 / TPAMI 2013).
//
// Self-guided variant (guide = input). Smooths low-contrast variation while
// preserving sharp structure — better edge behaviour than bilateral, and runs
// in O(N) thanks to box-filter integral arithmetic.
//
// For each output pixel q:
//   q = mean_a * I + mean_b
// where a = var_I / (var_I + eps) and b = mean_I (1 - a) inside the local
// window of radius r. Box filtering each intermediate gives the O(N) cost.

/// Separable sliding-window box filter on f32 data.
/// Replicates boundary samples (extends edge pixels outward).
fn box_filter_f32(
    src: &[f32],
    dst: &mut [f32],
    width: usize,
    height: usize,
    radius: usize,
) {
    let n = width * height;
    debug_assert_eq!(src.len(), n);
    debug_assert_eq!(dst.len(), n);

    if radius == 0 {
        dst.copy_from_slice(src);
        return;
    }

    let mut tmp = vec![0f32; n];
    let win = (radius * 2 + 1) as f32;

    // Horizontal pass: src -> tmp
    for y in 0..height {
        let row = y * width;
        let mut acc = 0f32;
        // Initial window: replicate index 0 for `radius` samples plus indices 0..=radius
        acc += radius as f32 * src[row];
        for k in 0..=radius.min(width - 1) {
            acc += src[row + k];
        }
        for x in 0..width {
            tmp[row + x] = acc / win;
            let left = if x >= radius { x - radius } else { 0 };
            let right = (x + radius + 1).min(width - 1);
            acc += src[row + right] - src[row + left];
        }
    }

    // Vertical pass: tmp -> dst
    for x in 0..width {
        let mut acc = 0f32;
        acc += radius as f32 * tmp[x];
        for k in 0..=radius.min(height - 1) {
            acc += tmp[k * width + x];
        }
        for y in 0..height {
            dst[y * width + x] = acc / win;
            let top = if y >= radius { y - radius } else { 0 };
            let bot = (y + radius + 1).min(height - 1);
            acc += tmp[bot * width + x] - tmp[top * width + x];
        }
    }
}

/// Apply self-guided filter to one f32 channel in-place.
/// `radius` is the window half-size; `eps` is the regularization term in the
/// same scale as the channel values squared (for [0,1] data, 1e-3 ~ 1e-2).
fn filter_channel(channel: &mut [f32], width: usize, height: usize, radius: usize, eps: f32) {
    let n = channel.len();
    let mut mean_i = vec![0f32; n];
    box_filter_f32(channel, &mut mean_i, width, height, radius);

    // I*I
    let mut ii = vec![0f32; n];
    for k in 0..n {
        ii[k] = channel[k] * channel[k];
    }
    let mut mean_ii = vec![0f32; n];
    box_filter_f32(&ii, &mut mean_ii, width, height, radius);

    // a = var_I / (var_I + eps)
    let mut a = vec![0f32; n];
    for k in 0..n {
        let var = (mean_ii[k] - mean_i[k] * mean_i[k]).max(0.0);
        a[k] = var / (var + eps);
    }

    // b = mean_I * (1 - a)   (self-guided p = I)
    let mut b = vec![0f32; n];
    for k in 0..n {
        b[k] = mean_i[k] * (1.0 - a[k]);
    }

    let mut mean_a = vec![0f32; n];
    let mut mean_b = vec![0f32; n];
    box_filter_f32(&a, &mut mean_a, width, height, radius);
    box_filter_f32(&b, &mut mean_b, width, height, radius);

    for k in 0..n {
        channel[k] = mean_a[k] * channel[k] + mean_b[k];
    }
}

/// Apply self-guided filter to an RGBA image. Alpha channel is passed through.
/// `radius`: window half-size in pixels. `eps`: regularization in [0..1] scale (squared intensity).
pub fn guided_filter_rgba(
    pixels: &[u8],
    width: usize,
    height: usize,
    radius: usize,
    eps: f32,
) -> Vec<u8> {
    let n = width * height;
    let mut chans: [Vec<f32>; 3] = [vec![0f32; n], vec![0f32; n], vec![0f32; n]];
    for k in 0..n {
        for c in 0..3 {
            chans[c][k] = pixels[k * 4 + c] as f32 / 255.0;
        }
    }
    for c in 0..3 {
        filter_channel(&mut chans[c], width, height, radius, eps);
    }

    let mut out = vec![0u8; pixels.len()];
    for k in 0..n {
        for c in 0..3 {
            out[k * 4 + c] = (chans[c][k] * 255.0).clamp(0.0, 255.0) as u8;
        }
        out[k * 4 + 3] = pixels[k * 4 + 3];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_filter_uniform_invariant() {
        let src = vec![0.5f32; 100];
        let mut dst = vec![0f32; 100];
        box_filter_f32(&src, &mut dst, 10, 10, 3);
        for &v in &dst {
            assert!((v - 0.5).abs() < 1e-6, "got {}", v);
        }
    }

    #[test]
    fn box_filter_smooths_impulse() {
        let mut src = vec![0f32; 100];
        src[5 * 10 + 5] = 1.0;
        let mut dst = vec![0f32; 100];
        box_filter_f32(&src, &mut dst, 10, 10, 2);
        // Center value should equal 1 / (5*5) = 0.04 with radius 2 (window 5x5)
        let center = dst[5 * 10 + 5];
        assert!((center - 1.0 / 25.0).abs() < 1e-5, "center={}", center);
        // Total mass conserved (modulo boundary replication)
        let sum: f32 = dst.iter().sum();
        assert!(sum > 0.95 && sum < 1.05, "sum={}", sum);
    }

    #[test]
    fn guided_filter_uniform_unchanged() {
        let pixels: Vec<u8> = (0..(20 * 20)).flat_map(|_| [128u8, 64, 200, 255]).collect();
        let out = guided_filter_rgba(&pixels, 20, 20, 4, 0.002);
        // Allow ±2 due to float rounding
        for k in 0..(20 * 20) {
            for c in 0..3 {
                let d = (out[k * 4 + c] as i32 - pixels[k * 4 + c] as i32).abs();
                assert!(d <= 2, "channel {} pixel {} drifted by {}", c, k, d);
            }
        }
    }

    #[test]
    fn guided_filter_preserves_alpha() {
        let pixels: Vec<u8> = (0..(15 * 15)).flat_map(|_| [200u8, 100, 50, 123]).collect();
        let out = guided_filter_rgba(&pixels, 15, 15, 3, 0.002);
        for k in 0..(15 * 15) {
            assert_eq!(out[k * 4 + 3], 123, "alpha changed at {}", k);
        }
    }

    #[test]
    fn guided_filter_smooths_noise_below_eps() {
        // Image with low-amplitude noise on top of a constant.
        let w = 30;
        let h = 30;
        let mut pixels = vec![0u8; w * h * 4];
        for k in 0..(w * h) {
            // Pseudo-random noise. Use wrapping arithmetic to keep this stable
            // in debug builds (cargo test) where overflow panics.
            let r = (k as u32).wrapping_mul(1103515245).wrapping_add(12345);
            let n = ((r >> 16) & 0x7fff) as i32 % 11 - 5; // -5..5
            let v = (128 + n).clamp(0, 255) as u8;
            pixels[k * 4] = v;
            pixels[k * 4 + 1] = v;
            pixels[k * 4 + 2] = v;
            pixels[k * 4 + 3] = 255;
        }
        // eps large enough to consider this noise "flat"
        let out = guided_filter_rgba(&pixels, w, h, 4, 0.01);
        // Compute std of input vs output
        let mean_in: f32 =
            (0..(w * h)).map(|k| pixels[k * 4] as f32).sum::<f32>() / (w * h) as f32;
        let mean_out: f32 = (0..(w * h)).map(|k| out[k * 4] as f32).sum::<f32>() / (w * h) as f32;
        let var_in: f32 = (0..(w * h))
            .map(|k| (pixels[k * 4] as f32 - mean_in).powi(2))
            .sum::<f32>()
            / (w * h) as f32;
        let var_out: f32 = (0..(w * h))
            .map(|k| (out[k * 4] as f32 - mean_out).powi(2))
            .sum::<f32>()
            / (w * h) as f32;
        // Output variance should be much lower than input variance
        assert!(
            var_out < var_in * 0.3,
            "var_in={}, var_out={}",
            var_in,
            var_out
        );
    }

    #[test]
    fn guided_filter_preserves_strong_edge() {
        // Step edge: left half black, right half white. eps small.
        let w = 40;
        let h = 20;
        let mut pixels = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v = if x < w / 2 { 0 } else { 255 };
                let k = y * w + x;
                pixels[k * 4] = v;
                pixels[k * 4 + 1] = v;
                pixels[k * 4 + 2] = v;
                pixels[k * 4 + 3] = 255;
            }
        }
        let out = guided_filter_rgba(&pixels, w, h, 3, 0.0001);
        // Sample a pixel deep in the dark side (x=2) and bright side (x=37)
        let y = 10;
        let dark = out[(y * w + 2) * 4];
        let bright = out[(y * w + 37) * 4];
        assert!(dark < 30, "dark drifted to {}", dark);
        assert!(bright > 225, "bright drifted to {}", bright);
    }
}
