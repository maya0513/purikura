mod beauty;
mod blemish;
mod compositor;
mod eye_warp;
mod filters;
mod guided_filter;
mod skin_mask;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn apply_filter(pixels: &[u8], _width: u32, _height: u32, filter: &str) -> Vec<u8> {
    let mut buf = pixels.to_vec();
    match filter {
        "grayscale" => filters::apply_grayscale(&mut buf),
        "sepia" => filters::apply_sepia(&mut buf),
        "vivid" => filters::apply_vivid(&mut buf),
        "soft" => filters::apply_soft(&mut buf),
        "warm" => filters::apply_warm(&mut buf),
        "cool" => filters::apply_cool(&mut buf),
        _ => {}
    }
    buf
}

#[wasm_bindgen]
pub fn compose_frame(photo: &[u8], frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    compositor::alpha_composite(photo, frame, width, height)
}

const MASK_FEATHER_RADIUS: usize = 4;

/// Build the skin mask used by both `apply_beauty` and `remove_blemish`.
/// Exposed so JS callers can compute it once and reuse it across calls.
///
/// `face_oval`: face-outline polygon as flat [x0, y0, x1, y1, ...] in 0..1.
/// `exclusions_packed`: packed sub-polygons to subtract (eyes, lips, ...);
/// format `[n_polys, len_0, x,y,..., len_1, x,y,...]`. May be empty.
#[wasm_bindgen]
pub fn build_skin_mask(
    pixels: &[u8],
    width: u32,
    height: u32,
    face_oval: &[f32],
    exclusions_packed: &[f32],
) -> Vec<u8> {
    skin_mask::build_mask(
        width as usize,
        height as usize,
        face_oval,
        exclusions_packed,
        pixels,
        MASK_FEATHER_RADIUS,
    )
}

/// Skin smoothing + プリクラ tone adjustment. Caller pre-computes `mask` via
/// `build_skin_mask` (or any other source) and passes it as an alpha buffer.
#[wasm_bindgen]
pub fn apply_beauty(
    pixels: &[u8],
    width: u32,
    height: u32,
    mask: &[u8],
    strength: f32,
) -> Vec<u8> {
    beauty::apply_beauty(pixels, width as usize, height as usize, mask, strength)
}

/// Blemish (dark-spot + redness) detection and inpainting, gated by `mask`.
#[wasm_bindgen]
pub fn remove_blemish(
    pixels: &[u8],
    width: u32,
    height: u32,
    mask: &[u8],
    strength: f32,
) -> Vec<u8> {
    blemish::remove_blemish(pixels, width as usize, height as usize, mask, strength)
}

/// Iris-centred eye enlargement.
/// `eyes`: flat [cx0, cy0, r0, cx1, cy1, r1, ...] all normalized to image
/// width (cy uses height). Each eye gets its own radius — typically iris
/// radius × 2.5.
#[wasm_bindgen]
pub fn enlarge_eyes(
    pixels: &[u8],
    width: u32,
    height: u32,
    eyes: &[f32],
    strength: f32,
) -> Vec<u8> {
    eye_warp::enlarge_eyes(pixels, width, height, eyes, strength)
}
