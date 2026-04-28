pub fn alpha_composite(photo: &[u8], frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    let len = (width * height * 4) as usize;
    assert_eq!(photo.len(), len, "photo buffer size mismatch");
    assert_eq!(frame.len(), len, "frame buffer size mismatch");

    let mut out = photo.to_vec();
    for i in (0..len).step_by(4) {
        let fa = frame[i + 3] as f32 / 255.0;
        if fa > 0.0 {
            let pa = 1.0 - fa;
            out[i] = (frame[i] as f32 * fa + photo[i] as f32 * pa) as u8;
            out[i + 1] = (frame[i + 1] as f32 * fa + photo[i + 1] as f32 * pa) as u8;
            out[i + 2] = (frame[i + 2] as f32 * fa + photo[i + 2] as f32 * pa) as u8;
            out[i + 3] = 255;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8, a: u8, w: u32, h: u32) -> Vec<u8> {
        let mut buf = vec![0u8; (w * h * 4) as usize];
        for chunk in buf.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
        buf
    }

    #[test]
    fn composite_transparent_frame_unchanged() {
        let photo = solid(100, 100, 100, 255, 2, 2);
        let frame = solid(255, 0, 0, 0, 2, 2);
        let result = alpha_composite(&photo, &frame, 2, 2);
        assert_eq!(result[0], 100);
        assert_eq!(result[1], 100);
        assert_eq!(result[2], 100);
    }

    #[test]
    fn composite_opaque_frame_overwrites() {
        let photo = solid(100, 100, 100, 255, 2, 2);
        let frame = solid(255, 0, 0, 255, 2, 2);
        let result = alpha_composite(&photo, &frame, 2, 2);
        assert_eq!(result[0], 255);
        assert_eq!(result[1], 0);
        assert_eq!(result[2], 0);
    }

}
