// Shared interpolation utilities used by eye_warp and face_slim.

#[inline]
pub(crate) fn smoothstep01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Catmull-Rom cubic kernel weight (B = 0, C = 0.5).
#[inline]
pub(crate) fn cubic_weight(t: f32) -> f32 {
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

/// Bicubic sample of a single colour channel at fractional pixel coords.
pub(crate) fn bicubic_sample(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_endpoints() {
        assert!((smoothstep01(0.0) - 0.0).abs() < 1e-6);
        assert!((smoothstep01(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_midpoint_is_half() {
        assert!((smoothstep01(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_clamps_outside_range() {
        assert!((smoothstep01(-1.0) - 0.0).abs() < 1e-6);
        assert!((smoothstep01(2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cubic_weight_sums_to_one() {
        // For a fractional offset t in [0,1), the 4 kernel taps sum to ~1.
        for frac in [0.0f32, 0.25, 0.5, 0.75] {
            let sum: f32 = (-1..=2).map(|i| cubic_weight(i as f32 - frac)).sum();
            assert!((sum - 1.0).abs() < 1e-5, "sum={} for frac={}", sum, frac);
        }
    }

    #[test]
    fn bicubic_at_integer_recovers_input() {
        let p: Vec<u8> = (0..10 * 10)
            .flat_map(|i| {
                let v = (i % 250) as u8;
                [v, v, v, 255]
            })
            .collect();
        let v = bicubic_sample(&p, 10, 10, 5.0, 5.0, 0);
        let actual = p[(5 * 10 + 5) * 4] as f32;
        assert!(
            (v - actual).abs() < 1e-3,
            "bicubic mismatch: {} vs {}",
            v,
            actual
        );
    }

    #[test]
    fn bicubic_clamps_at_boundary() {
        let p: Vec<u8> = (0..4 * 4).flat_map(|_| [200u8, 100, 50, 255]).collect();
        // Sampling outside the image boundary should not panic and clamp.
        let v = bicubic_sample(&p, 4, 4, -0.5, -0.5, 0);
        assert!(v >= 0.0 && v <= 255.0);
    }
}
