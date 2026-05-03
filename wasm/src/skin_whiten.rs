// プリクラ-style skin whitening.
//
// Design rationale:
//   35 % screen layer was too strong: shadow pixels were lifted to near-grey,
//   destroying the 3-D shape of the face (looked like white paint).
//   The fix is to keep the screen-blend approach but turn the white layer down
//   to 15 % so that only the upper half of the tonal range is visibly lifted.
//   Combined with 95 % chroma retention the result is a natural porcelain look:
//   skin texture and shadow gradients survive, colour stays warm.
//
//   Pipeline:
//     1. Keep 95 % of chroma — warmth and skin character survive.
//     2. Screen blend with a gentle white layer (15 %).
//        At 15 % a mid-tone pixel shifts ~+12 levels, a shadow ~+20 levels.
//        Enough to read as "brighter", not enough to flatten shadows.
//     3. Tiny cool tint (−2 R, +2 B) for the porcelain quality.
//     4. Blend by mask × strength.
//
// With this scheme a pixel (180, 140, 120) at full mask / 0.85 strength
// becomes (187, 155, 140) — warm and skin-like, just noticeably paler.

const SCREEN_WHITE: f32 = 255.0 * 0.15; // 15 % white layer — gentle lift only
const CHROMA_KEEP: f32 = 0.95; // keep 95 % of original saturation

pub fn whiten_skin(pixels: &[u8], w: u32, h: u32, mask: &[u8], strength: f32) -> Vec<u8> {
    let mut out = pixels.to_vec();
    let n = (w * h) as usize;
    let strength = strength.clamp(0.0, 1.0);
    for i in 0..n {
        let m = mask[i] as f32 / 255.0 * strength;
        if m < 0.01 {
            continue;
        }
        let r = pixels[i * 4] as f32;
        let g = pixels[i * 4 + 1] as f32;
        let b = pixels[i * 4 + 2] as f32;

        // 1. Very mild desaturation: preserve 90 % of the original colour.
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        let r2 = luma + (r - luma) * CHROMA_KEEP;
        let g2 = luma + (g - luma) * CHROMA_KEEP;
        let b2 = luma + (b - luma) * CHROMA_KEEP;

        // 2. Screen blend with moderate white layer — lifts brightness while
        //    keeping local contrast (texture / shadow gradients survive).
        let sr = 255.0 - (255.0 - r2) * (255.0 - SCREEN_WHITE) / 255.0;
        let sg = 255.0 - (255.0 - g2) * (255.0 - SCREEN_WHITE) / 255.0;
        let sb = 255.0 - (255.0 - b2) * (255.0 - SCREEN_WHITE) / 255.0;

        // 3. Tiny cool tint.
        let sr = sr - 2.0;
        let sb = sb + 2.0;

        // 4. Blend into output.
        out[i * 4] = (r + (sr - r) * m).clamp(0.0, 255.0) as u8;
        out[i * 4 + 1] = (g + (sg - g) * m).clamp(0.0, 255.0) as u8;
        out[i * 4 + 2] = (b + (sb - b) * m).clamp(0.0, 255.0) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_pixels(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        (0..w * h).flat_map(|_| [r, g, b, 255]).collect()
    }

    #[test]
    fn zero_mask_returns_input() {
        let pixels = flat_pixels(10, 10, 180, 140, 120);
        let mask = vec![0u8; 10 * 10];
        let out = whiten_skin(&pixels, 10, 10, &mask, 1.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn zero_strength_returns_input() {
        let pixels = flat_pixels(10, 10, 180, 140, 120);
        let mask = vec![255u8; 10 * 10];
        let out = whiten_skin(&pixels, 10, 10, &mask, 0.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn full_mask_brightens_skin() {
        let pixels = flat_pixels(10, 10, 180, 140, 120);
        let mask = vec![255u8; 10 * 10];
        let out = whiten_skin(&pixels, 10, 10, &mask, 1.0);
        assert!(out[0] > 180, "R didn't rise: {}", out[0]);
        assert!(out[1] > 140, "G didn't rise: {}", out[1]);
        assert!(out[2] > 120, "B didn't rise: {}", out[2]);
    }

    #[test]
    fn alpha_preserved() {
        let pixels: Vec<u8> = (0..10 * 10).flat_map(|_| [180u8, 140, 120, 128]).collect();
        let mask = vec![255u8; 10 * 10];
        let out = whiten_skin(&pixels, 10, 10, &mask, 1.0);
        for k in 0..(10 * 10) {
            assert_eq!(out[k * 4 + 3], 128);
        }
    }

    #[test]
    fn reduces_colour_saturation() {
        // Warm skin R >> G. After whitening the gap narrows (90 % chroma retention
        // still reduces absolute saturation because screen lift is colour-relative).
        let pixels = flat_pixels(10, 10, 200, 120, 100);
        let mask = vec![255u8; 10 * 10];
        let out = whiten_skin(&pixels, 10, 10, &mask, 1.0);
        let in_gap = pixels[0] as i32 - pixels[1] as i32;
        let out_gap = out[0] as i32 - out[1] as i32;
        assert!(
            out_gap < in_gap,
            "saturation not reduced: in_gap={in_gap}, out_gap={out_gap}"
        );
    }

    #[test]
    fn colour_character_preserved() {
        // After whitening, warm skin must still be warmer than neutral grey:
        // R > G > B relationship must survive (proves no "white paint" effect).
        let pixels = flat_pixels(10, 10, 180, 140, 120);
        let mask = vec![255u8; 10 * 10];
        let out = whiten_skin(&pixels, 10, 10, &mask, 0.85);
        assert!(
            out[0] > out[1],
            "warm R>G tone lost: R={} G={}",
            out[0],
            out[1]
        );
        assert!(
            out[1] > out[2],
            "warm G>B tone lost: G={} B={}",
            out[1],
            out[2]
        );
    }
}
