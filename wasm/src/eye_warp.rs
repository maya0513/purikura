// Iris-centred eye enlargement via sphere-lens warp.
//
// Each eye contributes its own (centre, radius) — radius is set by the caller
// to ~2.5 × iris radius so the warp doesn't reach as far as eyebrow / nose
// bridge / cheek (the failure mode of the previous radius-from-image-width
// approach, which produced a "cross-eyed" squashing).
//
// Falloff is smoothstep on (1 - d/R), giving a C¹-smooth falloff that hits
// exactly 0 at the boundary — no wraparound, no halo. Sampling uses
// Catmull-Rom bicubic for crisper iris detail than bilinear.

#[inline]
fn smoothstep01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Catmull-Rom cubic kernel weight (B = 0, C = 0.5).
#[inline]
fn cubic_weight(t: f32) -> f32 {
    let a = -0.5f32;
    let t = t.abs();
    if t < 1.0 {
        ((a + 2.0) * t - (a + 3.0)) * t * t + 1.0
    } else if t < 2.0 {
        ((a * t - 5.0 * a) * t + 8.0 * a) * t - 4.0 * a
    } else {
        0.0
    }
}

/// Bicubic sample of a single colour channel at fractional coords.
fn bicubic_sample(
    pixels: &[u8],
    width: usize,
    height: usize,
    x: f32,
    y: f32,
    channel: usize,
) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;

    let mut acc = 0f32;
    let mut wsum = 0f32;
    for j in -1i32..=2 {
        let wy = cubic_weight(j as f32 - fy);
        for i in -1i32..=2 {
            let wx = cubic_weight(i as f32 - fx);
            let sx = (xi + i).clamp(0, width as i32 - 1) as usize;
            let sy = (yi + j).clamp(0, height as i32 - 1) as usize;
            let v = pixels[(sy * width + sx) * 4 + channel] as f32;
            let w = wx * wy;
            acc += v * w;
            wsum += w;
        }
    }
    if wsum.abs() < 1e-6 {
        0.0
    } else {
        acc / wsum
    }
}

/// Enlarge each provided eye in place.
///
/// `eyes` is a flat slice of triples (cx, cy, r) all normalized to [0..1] of
/// the image width (cy uses height). `strength` ∈ [0..1].
pub fn enlarge_eyes(
    pixels: &[u8],
    width: u32,
    height: u32,
    eyes: &[f32],
    strength: f32,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut out = pixels.to_vec();

    let n_eyes = eyes.len() / 3;
    let strength = strength.clamp(0.0, 1.0);

    for i in 0..n_eyes {
        let cx = eyes[i * 3] * width as f32;
        let cy = eyes[i * 3 + 1] * height as f32;
        let radius = eyes[i * 3 + 2] * width as f32;
        if radius <= 1.0 {
            continue;
        }

        let x0 = (cx - radius).floor().max(0.0) as usize;
        let x1 = ((cx + radius).ceil() as i32).max(0) as usize;
        let x1 = x1.min(w);
        let y0 = (cy - radius).floor().max(0.0) as usize;
        let y1 = ((cy + radius).ceil() as i32).max(0) as usize;
        let y1 = y1.min(h);

        for py in y0..y1 {
            for px in x0..x1 {
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                let d = (dx * dx + dy * dy).sqrt();
                if d >= radius {
                    continue;
                }
                let t = d / radius;
                // Falloff: 1 at centre, smooth to 0 at boundary.
                let falloff = smoothstep01(1.0 - t);
                let scale = 1.0 - strength * falloff;

                // Sample input at centre + (output offset) × scale.
                // scale < 1 ⇒ source nearer the centre ⇒ output looks magnified.
                let sx = cx + dx * scale;
                let sy = cy + dy * scale;

                let dst = (py * w + px) * 4;
                for c in 0..3 {
                    let v = bicubic_sample(pixels, w, h, sx, sy, c);
                    out[dst + c] = v.clamp(0.0, 255.0) as u8;
                }
                out[dst + 3] = pixels[dst + 3];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient_image(w: u32, h: u32) -> Vec<u8> {
        let mut p = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let v = ((x + y) * 255 / (w + h - 2).max(1)) as u8;
                p.extend_from_slice(&[v, v, v, 200]);
            }
        }
        p
    }

    #[test]
    fn empty_eyes_returns_input() {
        let p = gradient_image(20, 20);
        let out = enlarge_eyes(&p, 20, 20, &[], 0.5);
        assert_eq!(out, p);
    }

    #[test]
    fn alpha_preserved() {
        let p: Vec<u8> = (0..20 * 20).flat_map(|_| [128u8, 128, 128, 137]).collect();
        let eyes = [0.5f32, 0.5, 0.2];
        let out = enlarge_eyes(&p, 20, 20, &eyes, 0.5);
        for i in (0..out.len()).step_by(4) {
            assert_eq!(out[i + 3], 137, "alpha drift at {}", i / 4);
        }
    }

    #[test]
    fn outside_radius_unchanged() {
        let p = gradient_image(40, 40);
        let eyes = [0.5f32, 0.5, 0.1]; // R = 4 px
        let out = enlarge_eyes(&p, 40, 40, &eyes, 0.5);
        // Pixel far from centre (corner) must be untouched.
        let i = 0;
        assert_eq!(out[i * 4], p[i * 4]);
        let last = (39 * 40 + 39) * 4;
        assert_eq!(out[last], p[last]);
    }

    #[test]
    fn bicubic_at_integer_recovers_input() {
        let p: Vec<u8> = (0..10 * 10).flat_map(|i| {
            let v = (i % 250) as u8;
            [v, v, v, 255]
        }).collect();
        let v = bicubic_sample(&p, 10, 10, 5.0, 5.0, 0);
        let actual = p[(5 * 10 + 5) * 4] as f32;
        assert!((v - actual).abs() < 1e-3, "bicubic mismatch: {} vs {}", v, actual);
    }

    #[test]
    fn warp_pulls_source_toward_centre() {
        // 7x7 bright square at centre, dark elsewhere. Magnifying this should
        // make pixels just outside the original bright square become non-zero.
        let w = 41u32;
        let h = 41u32;
        let mut p = vec![0u8; (w * h * 4) as usize];
        for k in 0..(w * h) as usize {
            p[k * 4 + 3] = 255;
        }
        let cx = 20i32;
        let cy = 20i32;
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                let x = (cx + dx) as usize;
                let y = (cy + dy) as usize;
                let i = (y * w as usize + x) * 4;
                p[i] = 255;
                p[i + 1] = 255;
                p[i + 2] = 255;
            }
        }
        let eyes = [0.5f32, 0.5, 0.3]; // R = 12 px (≈ 1.71 × bright radius)
        let out = enlarge_eyes(&p, w, h, &eyes, 0.7);
        // Probe a pixel that was outside the original bright square (5 right of centre).
        let probe = ((cy as usize) * w as usize + (cx as usize) + 5) * 4;
        assert_eq!(p[probe], 0, "test setup: probe must be black originally");
        assert!(
            out[probe] > 100,
            "warp didn't expand bright spot to (cx+5,cy): out={}",
            out[probe]
        );
    }

    #[test]
    fn zero_strength_is_identity() {
        // strength=0 ⇒ scale = 1 ⇒ source = output integer position. Bicubic
        // at integer coords recovers the exact pixel value (Catmull-Rom passes
        // through samples).
        let p = gradient_image(40, 40);
        let eyes = [0.5f32, 0.5, 0.2];
        let out = enlarge_eyes(&p, 40, 40, &eyes, 0.0);
        for k in 0..(40 * 40) {
            for c in 0..3 {
                let d = (out[k * 4 + c] as i32 - p[k * 4 + c] as i32).abs();
                assert!(d <= 1, "drift at {} ch {}: {}", k, c, d);
            }
        }
    }
}
