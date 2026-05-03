mod background;
mod beauty;
mod blemish;
mod color_overlay;
mod eye_sparkle;
mod eye_warp;
mod face_slim;
mod filters;
mod gpu;
mod guided_filter;
mod interpolation;
mod lut3d;
mod makeup;
mod skin_mask;
mod skin_whiten;

use wasm_bindgen::prelude::*;

// ── Re-export GPU init ────────────────────────────────────────────────────────

pub use gpu::init_gpu;

// ── Re-export GPU/async operations ───────────────────────────────────────────

pub use background::process_background;
pub use lut3d::apply_lut3d;

// ── Existing synchronous exports ─────────────────────────────────────────────

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

const MASK_FEATHER_RADIUS: usize = 4;

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

#[wasm_bindgen]
pub fn apply_beauty(pixels: &[u8], width: u32, height: u32, mask: &[u8], strength: f32) -> Vec<u8> {
    beauty::apply_beauty(pixels, width as usize, height as usize, mask, strength)
}

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

// ── New synchronous exports ───────────────────────────────────────────────────

#[wasm_bindgen]
pub fn whiten_skin(pixels: &[u8], width: u32, height: u32, mask: &[u8], strength: f32) -> Vec<u8> {
    skin_whiten::whiten_skin(pixels, width, height, mask, strength)
}

#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn apply_eye_sparkle(
    pixels: &[u8],
    width: u32,
    height: u32,
    eyes: &[f32],
    left_eye: &[f32],
    right_eye: &[f32],
    strength: f32,
) -> Vec<u8> {
    eye_sparkle::apply_eye_sparkle(pixels, width, height, eyes, left_eye, right_eye, strength)
}

#[wasm_bindgen]
pub fn slim_face(
    pixels: &[u8],
    width: u32,
    height: u32,
    face_oval: &[f32],
    strength: f32,
) -> Vec<u8> {
    face_slim::slim_face(pixels, width, height, face_oval, strength)
}

/// Apply makeup effects (lip / eye-shadow / blush).
///
/// `lips_outer`, `left_eye`, `right_eye`: normalised flat polygon coordinates.
/// `cheeks`: `[cx_left, cy_left, cx_right, cy_right]` normalised.
/// `params_json`: JSON string matching `MakeupParamsJson`.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn apply_makeup(
    pixels: &[u8],
    width: u32,
    height: u32,
    lips_outer: &[f32],
    left_eye: &[f32],
    right_eye: &[f32],
    cheeks: &[f32],
    params_json: &str,
) -> Vec<u8> {
    makeup::apply_makeup(
        pixels,
        width,
        height,
        lips_outer,
        left_eye,
        right_eye,
        cheeks,
        params_json,
    )
}

#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn apply_color_overlay(
    pixels: &[u8],
    width: u32,
    height: u32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
    blend_mode: &str,
    vignette: f32,
) -> Vec<u8> {
    color_overlay::apply_color_overlay(pixels, width, height, r, g, b, alpha, blend_mode, vignette)
}
