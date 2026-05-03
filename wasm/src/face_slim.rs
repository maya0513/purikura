// Face-slimming warp: horizontal squeeze based on face oval scan-line extent.
//
// Algorithm:
//   For each pixel inside the face oval, find the leftmost and rightmost x of
//   the oval at that scanline. Compute a normalised horizontal offset from the
//   face's vertical centre axis, and shift the source position outward so the
//   output looks narrower. The vertical contribution is shaped by a smooth
//   bell centred on the widest scanline (cheek level), falling off toward the
//   chin and forehead.
//
// Sampling uses the same Catmull-Rom bicubic from eye_warp.rs to match the
// sharpness of the eye-enlargement step.

use crate::interpolation::{bicubic_sample, smoothstep01};
use crate::skin_mask::point_in_polygon;

/// Return the leftmost and rightmost x-intersections of `poly_px` (pixel-space
/// flat [x0,y0,...]) with the horizontal line at `y`.  Returns None outside
/// the polygon's vertical extent.
fn oval_x_span(poly_px: &[f32], y: f32) -> Option<(f32, f32)> {
    let n = poly_px.len() / 2;
    if n < 3 {
        return None;
    }
    let mut xs = [f32::INFINITY, f32::NEG_INFINITY]; // [min, max]
    let mut found = 0usize;
    for i in 0..n {
        let x0 = poly_px[i * 2];
        let y0 = poly_px[i * 2 + 1];
        let x1 = poly_px[((i + 1) % n) * 2];
        let y1 = poly_px[((i + 1) % n) * 2 + 1];
        if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
            let t = (y - y0) / (y1 - y0 + f32::EPSILON);
            let xi = x0 + t * (x1 - x0);
            if xi < xs[0] {
                xs[0] = xi;
            }
            if xi > xs[1] {
                xs[1] = xi;
            }
            found += 1;
        }
    }
    if found >= 2 {
        Some((xs[0], xs[1]))
    } else {
        None
    }
}

/// Slim the face horizontally.
///
/// `face_oval`: flat normalized polygon [x0,y0,...] values in 0..1.
/// `strength` ∈ [0..1] — how much to squeeze (0.3 is already quite noticeable).
pub fn slim_face(
    pixels: &[u8],
    width: u32,
    height: u32,
    face_oval: &[f32],
    strength: f32,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let strength = strength.clamp(0.0, 1.0);

    if strength < 1e-6 || face_oval.len() < 6 {
        return pixels.to_vec();
    }

    // Convert face_oval to pixel space
    let n_oval = face_oval.len() / 2;
    let mut oval_px = Vec::with_capacity(n_oval * 2);
    for i in 0..n_oval {
        oval_px.push(face_oval[i * 2] * w as f32);
        oval_px.push(face_oval[i * 2 + 1] * h as f32);
    }

    // Find the scanline with the maximum face width (= cheek level)
    let (cheek_y, face_width_max) = {
        let mut best_y = h as f32 / 2.0;
        let mut best_w = 0.0f32;
        for py in 0..h {
            if let Some((lx, rx)) = oval_x_span(&oval_px, py as f32 + 0.5) {
                let fw = rx - lx;
                if fw > best_w {
                    best_w = fw;
                    best_y = py as f32 + 0.5;
                }
            }
        }
        (best_y, best_w)
    };

    // Face oval bounding box (for early exit)
    let oval_ymin = oval_px
        .chunks(2)
        .map(|c| c[1])
        .fold(f32::INFINITY, f32::min);
    let oval_ymax = oval_px
        .chunks(2)
        .map(|c| c[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let oval_xmin = oval_px
        .chunks(2)
        .map(|c| c[0])
        .fold(f32::INFINITY, f32::min);
    let oval_xmax = oval_px
        .chunks(2)
        .map(|c| c[0])
        .fold(f32::NEG_INFINITY, f32::max);

    // Flat face_oval as &[f32] for point_in_polygon
    let oval_flat: Vec<f32> = oval_px.clone();

    let mut out = pixels.to_vec();

    let y0 = (oval_ymin.floor() as i32).max(0) as usize;
    let y1 = ((oval_ymax.ceil() as i32 + 1) as usize).min(h);
    let x0 = (oval_xmin.floor() as i32).max(0) as usize;
    let x1 = ((oval_xmax.ceil() as i32 + 1) as usize).min(w);

    for py in y0..y1 {
        let yc = py as f32 + 0.5;

        // Vertical bell: maximum at cheek_y, tapering to 0 at face top/bottom.
        let vert_span = (oval_ymax - oval_ymin).max(1.0);
        let dy_norm = (yc - cheek_y).abs() / (vert_span * 0.5);
        let vert_weight = smoothstep01(1.0 - dy_norm);

        let span = match oval_x_span(&oval_px, yc) {
            Some(s) => s,
            None => continue,
        };
        let (lx, rx) = span;
        let cx_y = (lx + rx) / 2.0;
        let hw = (rx - lx) / 2.0;
        if hw < 1.0 {
            continue;
        }

        for px in x0..x1 {
            let xc = px as f32 + 0.5;
            if !point_in_polygon(xc, yc, &oval_flat) {
                continue;
            }

            let dx = xc - cx_y;
            let dn = dx / hw; // normalised: -1 (left boundary) .. +1 (right boundary)

            // The further from center, the more squeeze.
            // Source is pushed outward so face-content occupies less destination space.
            let squeeze = strength * vert_weight * dn.abs();
            let scale = 1.0 + squeeze; // > 1 → source farther from center → narrowing
            let sx = cx_y + dx * scale;
            let sy = yc;

            let dst = (py * w + px) * 4;
            for c in 0..3 {
                let v = bicubic_sample(pixels, w, h, sx, sy, c);
                out[dst + c] = v.clamp(0.0, 255.0) as u8;
            }
            out[dst + 3] = pixels[dst + 3];
        }
    }

    // Pixels outside the face oval are copied verbatim (already in `out`).
    // Pixels where `sx` falls outside the image clamp gracefully via bicubic_sample.
    let _ = face_width_max; // used only during cheek_y computation
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, v: u8) -> Vec<u8> {
        vec![v, v, v, 255].repeat((w * h) as usize)
    }

    fn gradient_h(w: u32, h: u32) -> Vec<u8> {
        let mut p = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..h {
            for x in 0..w {
                let v = (x * 255 / (w - 1)) as u8;
                p.extend_from_slice(&[v, v, v, 255]);
            }
        }
        p
    }

    fn full_oval(frac: f32) -> Vec<f32> {
        // Approximate ellipse with 8 points; covers center fraction of image.
        let half = frac / 2.0;
        let cx = 0.5f32;
        let cy = 0.5f32;
        let mut pts = Vec::new();
        let n = 8usize;
        for i in 0..n {
            let angle = (i as f32) * std::f32::consts::TAU / n as f32;
            pts.push(cx + half * angle.cos());
            pts.push(cy + half * angle.sin());
        }
        pts
    }

    #[test]
    fn zero_strength_identity() {
        let p = gradient_h(40, 40);
        let oval = full_oval(0.8);
        let out = slim_face(&p, 40, 40, &oval, 0.0);
        for k in 0..(40 * 40) {
            for c in 0..3 {
                let d = (out[k * 4 + c] as i32 - p[k * 4 + c] as i32).abs();
                assert!(
                    d <= 1,
                    "ch{c} at {k}: out={} p={}",
                    out[k * 4 + c],
                    p[k * 4 + c]
                );
            }
        }
    }

    #[test]
    fn alpha_preserved() {
        let p: Vec<u8> = (0..40 * 40).flat_map(|_| [128u8, 128, 128, 77]).collect();
        let oval = full_oval(0.8);
        let out = slim_face(&p, 40, 40, &oval, 0.4);
        for i in (3..out.len()).step_by(4) {
            assert_eq!(out[i], 77, "alpha drift at {i}");
        }
    }

    #[test]
    fn empty_oval_returns_input() {
        let p = solid(20, 20, 128);
        let out = slim_face(&p, 20, 20, &[], 0.5);
        assert_eq!(out, p);
    }

    #[test]
    fn outside_oval_unchanged() {
        let p = gradient_h(40, 40);
        let oval = full_oval(0.5); // only covers inner 50%
        let out = slim_face(&p, 40, 40, &oval, 0.5);
        // Corner pixel (0,0) should not be touched.
        assert_eq!(out[0], p[0]);
    }

    #[test]
    fn positive_strength_narrows_face() {
        // Use a horizontal gradient: left=0, right=255.
        // With strength > 0, a pixel near the right boundary of the oval
        // should sample from further right in the source (squeeze outward),
        // making it look darker (closer to 0) than the original.
        let w = 40u32;
        let h = 40u32;
        let p = gradient_h(w, h);
        let oval = full_oval(0.9);
        let out = slim_face(&p, w, h, &oval, 0.5);

        // Right-hand interior pixel (x≈34, y=20): original value ≈ 220.
        // Slimming pushes source rightward → value should increase toward 255.
        let px = 34usize;
        let py = 20usize;
        let idx = (py * w as usize + px) * 4;
        let orig_v = p[idx] as i32;
        let slim_v = out[idx] as i32;
        // After slimming, the right-side pixel maps from a source further right,
        // which in the gradient is brighter (higher value).
        assert!(
            slim_v >= orig_v - 2,
            "right-side pixel should not get darker: orig={orig_v}, slim={slim_v}"
        );
        // And the overall image must differ from the identity (slimming actually did something).
        let changed = (0..w * h)
            .filter(|&k| {
                let i = k as usize * 4;
                (out[i] as i32 - p[i] as i32).abs() > 2
            })
            .count();
        assert!(changed > 10, "too few pixels changed: {changed}");
    }
}
