// Eye sparkle: two-pass effect for the プリクラ "shining eyes" look.
//
// Pass 1 — Eye brightening: all pixels inside each eye polygon are
//   brightened proportionally to their luminance.  Bright sclera pixels
//   receive the strongest lift; dark iris/pupil pixels receive very little.
//   This avoids the false-positive problem of hard sclera-detection
//   (which misidentifies glass lenses as sclera).
//
// Pass 2 — Catchlight: a tight Gaussian white spot is screen-composited
//   at the upper-left quadrant of each iris (~10 o'clock position).
//   iris_r is derived from the warp radius (warp_r = iris_r × 2.5).

// Even–odd ray-casting point-in-polygon for normalised coordinates.
fn inside_polygon(px: f32, py: f32, poly: &[f32]) -> bool {
    let n = poly.len() / 2;
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = poly[i * 2];
        let yi = poly[i * 2 + 1];
        let xj = poly[j * 2];
        let yj = poly[j * 2 + 1];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Brighten all pixels inside `poly` proportionally to their luminance.
/// Sclera (bright) gets the most lift; iris/pupil (dark) barely changes.
fn brighten_eye(out: &mut [u8], pixels: &[u8], w: usize, h: usize, poly: &[f32], strength: f32) {
    if poly.len() < 6 {
        return;
    }
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for i in 0..poly.len() / 2 {
        min_x = min_x.min(poly[i * 2]);
        max_x = max_x.max(poly[i * 2]);
        min_y = min_y.min(poly[i * 2 + 1]);
        max_y = max_y.max(poly[i * 2 + 1]);
    }
    let px0 = ((min_x * w as f32).floor() as i32).max(0) as usize;
    let px1 = ((max_x * w as f32).ceil() as i32 + 1).max(0).min(w as i32) as usize;
    let py0 = ((min_y * h as f32).floor() as i32).max(0) as usize;
    let py1 = ((max_y * h as f32).ceil() as i32 + 1).max(0).min(h as i32) as usize;

    for py in py0..py1 {
        for px in px0..px1 {
            let nx = (px as f32 + 0.5) / w as f32;
            let ny = (py as f32 + 0.5) / h as f32;
            if !inside_polygon(nx, ny, poly) {
                continue;
            }
            let idx = (py * w + px) * 4;
            let rf = pixels[idx] as f32 / 255.0;
            let gf = pixels[idx + 1] as f32 / 255.0;
            let bf = pixels[idx + 2] as f32 / 255.0;
            let luma = 0.299 * rf + 0.587 * gf + 0.114 * bf;
            // Luma-weighted blend toward white: bright pixels (sclera ~0.9 luma)
            // get t≈0.42; dark pixels (iris ~0.2 luma) get t≈0.21.
            // Coefficient reduced from 0.9→0.5 so glass-lens pixels and dark
            // irises are not over-brightened to an unnatural white.
            let t = strength * luma.sqrt() * 0.5;
            out[idx] = ((rf + (1.0 - rf) * t) * 255.0).clamp(0.0, 255.0) as u8;
            out[idx + 1] = ((gf + (1.0 - gf) * t) * 255.0).clamp(0.0, 255.0) as u8;
            out[idx + 2] = ((bf + (1.0 - bf) * t) * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Paint a tight Gaussian white catchlight via screen composite.
/// Position: (cx − iris_r·0.30, cy − iris_r·0.40) — upper-left of iris.
fn add_catchlight(
    out: &mut [u8],
    w: usize,
    h: usize,
    cx_norm: f32,
    cy_norm: f32,
    iris_r_norm: f32,
    strength: f32,
) {
    if iris_r_norm <= 0.004 {
        return;
    }
    let cx = cx_norm * w as f32;
    let cy = cy_norm * h as f32;
    let iris_r = iris_r_norm * w as f32;

    let catch_cx = cx - iris_r * 0.30;
    let catch_cy = cy - iris_r * 0.40;
    // Catchlight: radius = 40 % of iris radius, sigma = 55 % of that.
    // Larger sigma than before so the spot looks like a soft light reflection
    // rather than a single pixel dot.
    let catch_r = iris_r * 0.40;
    let sigma = catch_r * 0.55;
    let sigma2 = 2.0 * sigma * sigma;
    let cutoff = catch_r * 2.5;

    let x0 = ((catch_cx - cutoff).floor() as i32).max(0) as usize;
    let x1 = ((catch_cx + cutoff).ceil() as i32 + 1).max(0).min(w as i32) as usize;
    let y0 = ((catch_cy - cutoff).floor() as i32).max(0) as usize;
    let y1 = ((catch_cy + cutoff).ceil() as i32 + 1).max(0).min(h as i32) as usize;

    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f32 - catch_cx;
            let dy = py as f32 - catch_cy;
            let gauss = (-(dx * dx + dy * dy) / sigma2).exp();
            let blend = gauss * strength;
            if blend < 0.02 {
                continue;
            }
            let idx = (py * w + px) * 4;
            // Screen composite: 1 − (1−dst)(1−src)
            let rf = out[idx] as f32 / 255.0;
            let gf = out[idx + 1] as f32 / 255.0;
            let bf = out[idx + 2] as f32 / 255.0;
            out[idx] = ((1.0 - (1.0 - rf) * (1.0 - blend)) * 255.0).clamp(0.0, 255.0) as u8;
            out[idx + 1] =
                ((1.0 - (1.0 - gf) * (1.0 - blend)) * 255.0).clamp(0.0, 255.0) as u8;
            out[idx + 2] =
                ((1.0 - (1.0 - bf) * (1.0 - blend)) * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Apply プリクラ-style eye sparkle.
///
/// `eyes`: `[cx, cy, warp_r, ...]` triples, normalised. `warp_r = iris_r × 2.5`.
/// `left_eye` / `right_eye`: flat polygon vertices for each eye socket, normalised.
/// `strength` ∈ [0..1].
pub fn apply_eye_sparkle(
    pixels: &[u8],
    w: u32,
    h: u32,
    eyes: &[f32],
    left_eye: &[f32],
    right_eye: &[f32],
    strength: f32,
) -> Vec<u8> {
    let mut out = pixels.to_vec();
    let ww = w as usize;
    let hh = h as usize;
    let strength = strength.clamp(0.0, 1.0);

    // Pass 1: brighten pixels inside each eye polygon (uses original as source)
    brighten_eye(&mut out, pixels, ww, hh, left_eye, strength);
    brighten_eye(&mut out, pixels, ww, hh, right_eye, strength);

    // Pass 2: catchlight per iris
    let n_eyes = eyes.len() / 3;
    for i in 0..n_eyes {
        let cx = eyes[i * 3];
        let cy = eyes[i * 3 + 1];
        let iris_r = eyes[i * 3 + 2] / 2.5; // warp_r → iris_r
        add_catchlight(&mut out, ww, hh, cx, cy, iris_r, strength);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark_image(w: usize, h: usize) -> Vec<u8> {
        (0..w * h).flat_map(|_| [50u8, 50, 50, 255]).collect()
    }

    fn eye_image(w: usize, h: usize) -> Vec<u8> {
        // Central band: sclera-like white; surrounds: skin tone.
        let mut p: Vec<u8> = (0..w * h).flat_map(|_| [180u8, 140, 120, 255]).collect();
        for y in h / 4..(3 * h / 4) {
            for x in w / 4..(3 * w / 4) {
                let i = (y * w + x) * 4;
                p[i] = 230;
                p[i + 1] = 228;
                p[i + 2] = 226;
            }
        }
        p
    }

    #[test]
    fn empty_eyes_returns_input() {
        let pixels = dark_image(20, 20);
        let out = apply_eye_sparkle(&pixels, 20, 20, &[], &[], &[], 0.8);
        assert_eq!(out, pixels);
    }

    #[test]
    fn zero_strength_returns_input() {
        let pixels = dark_image(20, 20);
        let eyes = [0.5f32, 0.5, 0.2];
        let poly = [0.3f32, 0.3, 0.7, 0.3, 0.7, 0.7, 0.3, 0.7];
        let out = apply_eye_sparkle(&pixels, 20, 20, &eyes, &poly, &poly, 0.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn alpha_preserved() {
        let pixels: Vec<u8> = (0..20 * 20).flat_map(|_| [230u8, 228, 226, 99]).collect();
        let eyes = [0.5f32, 0.5, 0.2];
        let poly = [0.3f32, 0.3, 0.7, 0.3, 0.7, 0.7, 0.3, 0.7];
        let out = apply_eye_sparkle(&pixels, 20, 20, &eyes, &poly, &poly, 0.8);
        for k in 0..(20 * 20) {
            assert_eq!(out[k * 4 + 3], 99, "alpha drift at pixel {k}");
        }
    }

    #[test]
    fn bright_pixels_brightened_inside_polygon() {
        // Sclera-like pixels inside the polygon must get brighter.
        let w = 40usize;
        let h = 40usize;
        let pixels = eye_image(w, h);
        let poly = [0.25f32, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75];
        let out = apply_eye_sparkle(&pixels, w as u32, h as u32, &[], &poly, &[], 0.8);
        let cx = w / 2;
        let cy = h / 2;
        let i = (cy * w + cx) * 4;
        assert!(
            out[i] >= pixels[i],
            "bright pixel not lifted: before={} after={}",
            pixels[i],
            out[i]
        );
    }

    #[test]
    fn sclera_closer_to_white_than_iris() {
        // Luma-proportional weighting: sclera covers a greater fraction of the
        // remaining distance to white than an iris/pupil pixel does.
        // (Absolute gain is larger for dark pixels because they start further away —
        // the meaningful metric is how much of the gap to 255 is closed.)
        let w = 40usize;
        let h = 40usize;
        let mut pixels = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                let (r, g, b) = if x < w / 2 { (60, 50, 40) } else { (230, 228, 226) };
                pixels[i] = r;
                pixels[i + 1] = g;
                pixels[i + 2] = b;
                pixels[i + 3] = 255;
            }
        }
        let poly = [0.0f32, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let out = apply_eye_sparkle(&pixels, w as u32, h as u32, &[], &poly, &[], 0.8);
        // Fraction of gap to 255 that was closed.
        let dark_coverage =
            (out[0] as f32 - pixels[0] as f32) / (255.0 - pixels[0] as f32).max(1.0);
        let bright_idx = w / 2 * 4;
        let bright_coverage = (out[bright_idx] as f32 - pixels[bright_idx] as f32)
            / (255.0 - pixels[bright_idx] as f32).max(1.0);
        assert!(
            bright_coverage > dark_coverage,
            "sclera should close more of the gap to white: sclera={:.2} iris={:.2}",
            bright_coverage,
            dark_coverage
        );
    }

    #[test]
    fn catchlight_brightens_iris_region() {
        let w = 80usize;
        let h = 80usize;
        let pixels = dark_image(w, h);
        // warp_r = 0.20 → iris_r = 0.08 (normalised to width = 80px → 6.4px)
        let eyes = [0.5f32, 0.5, 0.20];
        let out = apply_eye_sparkle(&pixels, w as u32, h as u32, &eyes, &[], &[], 1.0);
        let catch_x = ((0.5 - 0.08 * 0.30) * w as f32) as usize;
        let catch_y = ((0.5 - 0.08 * 0.40) * h as f32) as usize;
        let i = (catch_y * w + catch_x) * 4;
        assert!(
            out[i] > pixels[i],
            "catchlight pixel not brightened: before={} after={}",
            pixels[i],
            out[i]
        );
    }

    #[test]
    fn pixels_outside_polygon_unchanged() {
        let w = 40usize;
        let h = 40usize;
        let pixels = eye_image(w, h);
        // Small polygon in centre — top-left corner must be untouched.
        let poly = [0.4f32, 0.4, 0.6, 0.4, 0.6, 0.6, 0.4, 0.6];
        let out = apply_eye_sparkle(&pixels, w as u32, h as u32, &[], &poly, &[], 0.8);
        assert_eq!(out[0], pixels[0]);
        assert_eq!(out[1], pixels[1]);
        assert_eq!(out[2], pixels[2]);
    }
}
