pub fn apply_grayscale(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let luma = (chunk[0] as f32 * 0.299
            + chunk[1] as f32 * 0.587
            + chunk[2] as f32 * 0.114) as u8;
        chunk[0] = luma;
        chunk[1] = luma;
        chunk[2] = luma;
    }
}

pub fn apply_sepia(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let r = chunk[0] as f32;
        let g = chunk[1] as f32;
        let b = chunk[2] as f32;
        chunk[0] = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0) as u8;
        chunk[1] = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0) as u8;
        chunk[2] = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0) as u8;
    }
}

pub fn apply_vivid(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;
        let (h, s, l) = rgb_to_hsl(r, g, b);
        let s2 = (s * 1.4).min(1.0);
        let (r2, g2, b2) = hsl_to_rgb(h, s2, l);
        chunk[0] = (r2 * 255.0) as u8;
        chunk[1] = (g2 * 255.0) as u8;
        chunk[2] = (b2 * 255.0) as u8;
    }
}

pub fn apply_soft(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = ((chunk[0] as f32 * 0.85 + 128.0 * 0.15) as u8).saturating_add(8);
        chunk[1] = ((chunk[1] as f32 * 0.85 + 128.0 * 0.15) as u8).saturating_add(8);
        chunk[2] = ((chunk[2] as f32 * 0.85 + 128.0 * 0.15) as u8).saturating_add(8);
    }
}

pub fn apply_warm(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = chunk[0].saturating_add(15);
        chunk[1] = chunk[1].saturating_add(5);
        chunk[2] = chunk[2].saturating_sub(15);
    }
}

pub fn apply_cool(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = chunk[0].saturating_sub(15);
        chunk[2] = chunk[2].saturating_add(20);
    }
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < f32::EPSILON {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    (hue_to_rgb(p, q, h + 1.0 / 3.0), hue_to_rgb(p, q, h), hue_to_rgb(p, q, h - 1.0 / 3.0))
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grayscale_white_stays_white() {
        let mut buf = vec![255u8, 255, 255, 255];
        apply_grayscale(&mut buf);
        assert_eq!(buf, [255, 255, 255, 255]);
    }

    #[test]
    fn grayscale_black_stays_black() {
        let mut buf = vec![0u8, 0, 0, 255];
        apply_grayscale(&mut buf);
        assert_eq!(buf, [0, 0, 0, 255]);
    }

    #[test]
    fn grayscale_preserves_alpha() {
        let mut buf = vec![128u8, 64, 32, 200];
        apply_grayscale(&mut buf);
        assert_eq!(buf[3], 200);
    }

    #[test]
    fn grayscale_rgb_equal_after() {
        let mut buf = vec![100u8, 150, 200, 255];
        apply_grayscale(&mut buf);
        assert_eq!(buf[0], buf[1]);
        assert_eq!(buf[1], buf[2]);
    }

    #[test]
    fn sepia_output_in_range() {
        let mut buf = vec![100u8, 100, 100, 255];
        apply_sepia(&mut buf);
        // sepia values are non-zero and alpha preserved
        assert!(buf[0] > 0);
        assert_eq!(buf[3], 255);
    }

    #[test]
    fn sepia_white_blue_reduced() {
        let mut buf = vec![255u8, 255, 255, 255];
        apply_sepia(&mut buf);
        // sepia of white: red and green clamp to 255, blue is reduced
        assert_eq!(buf[0], 255);
        assert_eq!(buf[1], 255);
        assert!(buf[2] < 255);
    }

    #[test]
    fn warm_increases_red() {
        let mut buf = vec![100u8, 100, 100, 255];
        apply_warm(&mut buf);
        assert_eq!(buf[0], 115);
        assert_eq!(buf[1], 105);
        assert_eq!(buf[2], 85);
    }

    #[test]
    fn warm_saturates_at_255() {
        let mut buf = vec![250u8, 254, 10, 255];
        apply_warm(&mut buf);
        assert_eq!(buf[0], 255);
        assert_eq!(buf[2], 0);
    }

    #[test]
    fn cool_increases_blue() {
        let mut buf = vec![100u8, 100, 100, 255];
        apply_cool(&mut buf);
        assert_eq!(buf[0], 85);
        assert_eq!(buf[2], 120);
    }

    #[test]
    fn soft_raises_midtones() {
        let mut buf = vec![0u8, 0, 0, 255];
        apply_soft(&mut buf);
        assert!(buf[0] > 0);
    }

    #[test]
    fn vivid_preserves_alpha() {
        let mut buf = vec![100u8, 150, 200, 128];
        apply_vivid(&mut buf);
        assert_eq!(buf[3], 128);
    }
}
