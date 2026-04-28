// Skin mask builder.
//
// Goal: identify exactly the facial skin pixels (and nothing else — no hair, no
// teeth, no lips, no clothing, no background). Three layers:
//   1. Geometric: rasterize the face oval polygon.
//   2. Subtract feature polygons (eyes, eyebrows, lips, nostrils).
//   3. Refine with YCbCr skin-color check (Kovac/Peer 2003 ranges) — pixels
//      inside the polygon that fall outside skin chroma are dimmed, not
//      removed, so that lipstick or shadow doesn't punch holes.
//   4. Feather the boundary (separable box blur ≈ Gaussian) so the smoothing
//      doesn't show a hard edge against unprocessed regions.
//
// Output is a u8 mask in [0..255] used as a per-pixel blend weight.

const INSIDE_BASE: u8 = 255;
const INSIDE_NONSKIN_DIM: u8 = 160; // when face polygon says "skin" but YCbCr says "no" — partial weight

/// Even-odd ray casting point-in-polygon test.
/// `poly` is a flat slice [x0, y0, x1, y1, ...] with values in any units.
/// `(x, y)` must be in the same units.
pub fn point_in_polygon(x: f32, y: f32, poly: &[f32]) -> bool {
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
        let cond1 = (yi > y) != (yj > y);
        let cond2 = x < (xj - xi) * (y - yi) / (yj - yi + f32::EPSILON) + xi;
        if cond1 && cond2 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Convert normalized polygon (values in 0..1) to pixel coordinates.
fn denormalize(poly_norm: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(poly_norm.len());
    for i in (0..poly_norm.len()).step_by(2) {
        out.push(poly_norm[i] * width as f32);
        out.push(poly_norm[i + 1] * height as f32);
    }
    out
}

/// Rasterize a polygon into a boolean mask. Pixel center used as test point.
fn rasterize_polygon(width: usize, height: usize, poly_pixels: &[f32]) -> Vec<bool> {
    let mut out = vec![false; width * height];
    if poly_pixels.len() < 6 {
        return out;
    }
    // Compute bbox to skip empty pixels.
    let mut xmin = f32::INFINITY;
    let mut xmax = f32::NEG_INFINITY;
    let mut ymin = f32::INFINITY;
    let mut ymax = f32::NEG_INFINITY;
    for i in (0..poly_pixels.len()).step_by(2) {
        xmin = xmin.min(poly_pixels[i]);
        xmax = xmax.max(poly_pixels[i]);
        ymin = ymin.min(poly_pixels[i + 1]);
        ymax = ymax.max(poly_pixels[i + 1]);
    }
    let x0 = (xmin.floor() as i32).max(0) as usize;
    let x1 = ((xmax.ceil() as i32 + 1).max(0) as usize).min(width);
    let y0 = (ymin.floor() as i32).max(0) as usize;
    let y1 = ((ymax.ceil() as i32 + 1).max(0) as usize).min(height);

    for y in y0..y1 {
        for x in x0..x1 {
            if point_in_polygon(x as f32 + 0.5, y as f32 + 0.5, poly_pixels) {
                out[y * width + x] = true;
            }
        }
    }
    out
}

#[inline]
fn is_skin_ycbcr(r: u8, g: u8, b: u8) -> bool {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let cb = -0.168736 * r - 0.331264 * g + 0.5 * b + 128.0;
    let cr = 0.5 * r - 0.418688 * g - 0.081312 * b + 128.0;
    y > 60.0 && (77.0..=130.0).contains(&cb) && (130.0..=180.0).contains(&cr)
}

/// One pass of separable box blur. Three passes ≈ Gaussian (CLT).
fn box_blur_pass(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 {
        dst.copy_from_slice(src);
        return;
    }
    // Horizontal
    let mut tmp = vec![0u16; width * height];
    let win = (radius * 2 + 1) as u32;
    for y in 0..height {
        let row = y * width;
        let mut acc: u32 = 0;
        // Initialize window: replicate edges
        for k in 0..=radius.min(width - 1) {
            acc += src[row + k] as u32;
        }
        // Left padding (replicate index 0)
        acc += (radius as u32) * src[row] as u32;
        for x in 0..width {
            tmp[row + x] = (acc / win) as u16;
            // Slide: subtract leftmost (with replicate at boundary), add right
            let left_idx = if x >= radius { x - radius } else { 0 };
            let right_idx = (x + radius + 1).min(width - 1);
            acc = acc + src[row + right_idx] as u32 - src[row + left_idx] as u32;
        }
    }
    // Vertical
    for x in 0..width {
        let mut acc: u32 = 0;
        for k in 0..=radius.min(height - 1) {
            acc += tmp[k * width + x] as u32;
        }
        acc += (radius as u32) * tmp[x] as u32;
        for y in 0..height {
            dst[y * width + x] = (acc / win) as u8;
            let top_idx = if y >= radius { y - radius } else { 0 };
            let bot_idx = (y + radius + 1).min(height - 1);
            acc = acc + tmp[bot_idx * width + x] as u32 - tmp[top_idx * width + x] as u32;
        }
    }
}

/// Triple box-blur ≈ Gaussian feathering of mask edges.
fn feather(mask: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 {
        return;
    }
    let mut tmp = vec![0u8; mask.len()];
    box_blur_pass(mask, &mut tmp, width, height, radius);
    box_blur_pass(&tmp, mask, width, height, radius);
    box_blur_pass(mask, &mut tmp, width, height, radius);
    mask.copy_from_slice(&tmp);
}

/// Decode a packed exclusions buffer.
/// Format: [n_polys, len_0, x0,y0,x1,y1,..., len_1, x0,y0,...]
fn iter_packed_polys(packed: &[f32]) -> Vec<&[f32]> {
    let mut out = Vec::new();
    if packed.is_empty() {
        return out;
    }
    let n = packed[0] as usize;
    let mut i = 1usize;
    for _ in 0..n {
        if i >= packed.len() {
            break;
        }
        let len = packed[i] as usize;
        i += 1;
        let coord_count = len * 2;
        if i + coord_count > packed.len() {
            break;
        }
        out.push(&packed[i..i + coord_count]);
        i += coord_count;
    }
    out
}

/// Build the skin mask.
///
/// `face_oval`: normalized polygon of the face oval (values in 0..1).
/// `exclusions_packed`: packed sub-polygons (eyes, eyebrows, lips, etc.) to subtract.
/// `pixels`: RGBA of the source image (used for YCbCr refinement).
/// `feather_radius`: half-window of the box blur passes used for edge feathering.
pub fn build_mask(
    width: usize,
    height: usize,
    face_oval: &[f32],
    exclusions_packed: &[f32],
    pixels: &[u8],
    feather_radius: usize,
) -> Vec<u8> {
    let mut mask = vec![0u8; width * height];

    let face_pixels = denormalize(face_oval, width, height);
    let face_bool = rasterize_polygon(width, height, &face_pixels);

    // Initial pass: inside face polygon, refine with YCbCr.
    for i in 0..(width * height) {
        if face_bool[i] {
            let r = pixels[i * 4];
            let g = pixels[i * 4 + 1];
            let b = pixels[i * 4 + 2];
            mask[i] = if is_skin_ycbcr(r, g, b) {
                INSIDE_BASE
            } else {
                INSIDE_NONSKIN_DIM
            };
        }
    }

    // Subtract exclusion polygons.
    for poly in iter_packed_polys(exclusions_packed) {
        let poly_pixels = denormalize(poly, width, height);
        let excl = rasterize_polygon(width, height, &poly_pixels);
        for i in 0..mask.len() {
            if excl[i] {
                mask[i] = 0;
            }
        }
    }

    // Feather the edges so smoothing doesn't show a hard mask boundary.
    feather(&mut mask, width, height, feather_radius);

    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba_uniform(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(w * h * 4);
        for _ in 0..(w * h) {
            out.extend_from_slice(&[r, g, b, 255]);
        }
        out
    }

    #[test]
    fn point_in_square() {
        let sq = [0.0f32, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        assert!(point_in_polygon(5.0, 5.0, &sq));
        assert!(!point_in_polygon(15.0, 5.0, &sq));
        assert!(!point_in_polygon(-1.0, 5.0, &sq));
    }

    #[test]
    fn point_in_triangle() {
        let tri = [0.0f32, 0.0, 10.0, 0.0, 5.0, 10.0];
        assert!(point_in_polygon(5.0, 1.0, &tri));
        assert!(point_in_polygon(5.0, 5.0, &tri));
        assert!(!point_in_polygon(0.5, 5.0, &tri));
        assert!(!point_in_polygon(9.5, 5.0, &tri));
    }

    #[test]
    fn rasterize_full_square() {
        // Polygon covering the whole 10x10 image
        let poly = [0.0f32, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        let m = rasterize_polygon(10, 10, &poly);
        assert!(m.iter().all(|&v| v));
    }

    #[test]
    fn rasterize_outside_square() {
        let poly = [100.0f32, 100.0, 110.0, 100.0, 110.0, 110.0, 100.0, 110.0];
        let m = rasterize_polygon(10, 10, &poly);
        assert!(m.iter().all(|&v| !v));
    }

    #[test]
    fn rasterize_triangle_half_filled() {
        let poly = [0.0f32, 0.0, 100.0, 0.0, 50.0, 100.0];
        let m = rasterize_polygon(100, 100, &poly);
        let count = m.iter().filter(|&&v| v).count();
        // Triangle area = 0.5 * 100 * 100 = 5000 pixels
        assert!((count as i32 - 5000).abs() < 250, "got {}", count);
    }

    #[test]
    fn build_mask_full_face_no_exclusions() {
        // Face oval = full image, exclusions empty, pixels = pure skin tone
        let face = [0.0f32, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let pixels = rgba_uniform(20, 20, 220, 180, 150);
        let mask = build_mask(20, 20, &face, &[], &pixels, 0);
        // All inside, all skin → 255
        assert!(mask.iter().all(|&v| v == 255), "first={}", mask[0]);
    }

    #[test]
    fn build_mask_excludes_eye_polygon() {
        // Face = full, exclusion = small square in center
        let face = [0.0f32, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let pixels = rgba_uniform(20, 20, 220, 180, 150);
        let excl = [
            1.0f32, // n_polys
            4.0,    // len_0
            0.4, 0.4, 0.6, 0.4, 0.6, 0.6, 0.4, 0.6,
        ];
        let mask = build_mask(20, 20, &face, &excl, &pixels, 0);
        // Center pixel must be 0
        let center = 10 * 20 + 10;
        assert_eq!(mask[center], 0, "center should be excluded");
        // Corner pixel must be inside skin
        assert_eq!(mask[0], 255);
    }

    #[test]
    fn build_mask_zero_outside_face() {
        // Small face polygon at center, rest of image = 0
        let face = [0.4f32, 0.4, 0.6, 0.4, 0.6, 0.6, 0.4, 0.6];
        let pixels = rgba_uniform(20, 20, 220, 180, 150);
        let mask = build_mask(20, 20, &face, &[], &pixels, 0);
        // Corner: outside face
        assert_eq!(mask[0], 0);
        assert_eq!(mask[19], 0);
        // Center: inside face, skin tone
        let center = 10 * 20 + 10;
        assert_eq!(mask[center], 255);
    }

    #[test]
    fn build_mask_dims_non_skin_inside_face() {
        let face = [0.0f32, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        // Pure blue: inside face polygon but not skin tone
        let pixels = rgba_uniform(10, 10, 0, 0, 255);
        let mask = build_mask(10, 10, &face, &[], &pixels, 0);
        // Should be dimmed (INSIDE_NONSKIN_DIM = 160), not 0 nor 255
        assert_eq!(mask[0], INSIDE_NONSKIN_DIM);
    }

    #[test]
    fn box_blur_uniform_is_invariant() {
        let mut src = vec![100u8; 100];
        let mut dst = vec![0u8; 100];
        box_blur_pass(&mut src, &mut dst, 10, 10, 2);
        for &v in &dst {
            assert!(v >= 99 && v <= 101, "uniform must stay uniform, got {}", v);
        }
    }

    #[test]
    fn feather_smooths_edge() {
        // Half-mask: left half 255, right half 0
        let w = 40;
        let h = 10;
        let mut m = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..(w / 2) {
                m[y * w + x] = 255;
            }
        }
        feather(&mut m, w, h, 2);
        // Around the boundary x=20, expect intermediate values, not just 0/255
        let mid = 5 * w + 20;
        assert!(m[mid] > 0 && m[mid] < 255, "edge mid value = {}", m[mid]);
        // Far left stays high, far right stays low
        assert!(m[5 * w + 1] > 200);
        assert!(m[5 * w + 38] < 60);
    }

    #[test]
    fn iter_packed_polys_decodes() {
        // 2 polygons: triangle then square
        let packed = [
            2.0f32, // n_polys
            3.0,    // len 0
            0.0, 0.0, 1.0, 0.0, 0.5, 1.0, // tri
            4.0,    // len 1
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, // sq
        ];
        let polys = iter_packed_polys(&packed);
        assert_eq!(polys.len(), 2);
        assert_eq!(polys[0].len(), 6);
        assert_eq!(polys[1].len(), 8);
    }
}
