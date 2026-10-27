use crate::model::Light;
use std::f32::consts::TAU;

/// 22pt menu-bar icon at 2x.
pub const ICON_PX: u32 = 44;

const CENTER: f32 = ICON_PX as f32 / 2.0;
const DISC_R: f32 = 18.0;
const RING_OUTER: f32 = 20.0;
const RING_INNER: f32 = 14.0;
const TRACK: [u8; 3] = [0x55, 0x55, 0x55];

fn rgb(light: Light) -> [u8; 3] {
    match light {
        Light::Green => [0x2E, 0xA0, 0x43],
        Light::Yellow => [0xD2, 0x99, 0x22],
        Light::Red => [0xF8, 0x51, 0x49],
        Light::Gray => [0x8B, 0x94, 0x9E],
    }
}

/// Fraction of a pixel covered by the band [inner, outer] around the center (cheap AA).
fn coverage(r: f32, inner: f32, outer: f32) -> f32 {
    ((outer - r + 0.5).min(r - inner + 0.5)).clamp(0.0, 1.0)
}

/// RGBA buffer: a filled disc, or with `progress` a ring filled clockwise from 12 o'clock.
pub fn render(light: Light, progress: Option<f32>) -> Vec<u8> {
    let color = rgb(light);
    let filled = progress.map(|p| (p.clamp(0.0, 1.0) * 8.0).ceil() / 8.0);
    let mut buf = vec![0u8; (ICON_PX * ICON_PX * 4) as usize];

    for y in 0..ICON_PX {
        for x in 0..ICON_PX {
            let dx = x as f32 + 0.5 - CENTER;
            let dy = y as f32 + 0.5 - CENTER;
            let r = (dx * dx + dy * dy).sqrt();
            let (alpha, c) = match filled {
                None => (coverage(r, -1.0, DISC_R), color),
                Some(f) => {
                    let angle = dx.atan2(-dy).rem_euclid(TAU) / TAU;
                    (coverage(r, RING_INNER, RING_OUTER), if angle < f { color } else { TRACK })
                }
            };
            if alpha > 0.0 {
                let i = ((y * ICON_PX + x) * 4) as usize;
                buf[i..i + 3].copy_from_slice(&c);
                buf[i + 3] = (alpha * 255.0).round() as u8;
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(b: &[u8], x: u32, y: u32) -> &[u8] {
        let i = ((y * ICON_PX + x) * 4) as usize;
        &b[i..i + 4]
    }

    #[test]
    fn buffer_size() {
        assert_eq!(render(Light::Green, None).len(), (ICON_PX * ICON_PX * 4) as usize);
    }

    #[test]
    fn center_pixel_has_color() {
        let b = render(Light::Red, None);
        assert_eq!(&px(&b, 22, 22)[..3], &[0xF8, 0x51, 0x49]);
        assert_eq!(px(&b, 22, 22)[3], 255);
    }

    #[test]
    fn corner_is_transparent() {
        assert_eq!(px(&render(Light::Green, None), 0, 0)[3], 0);
    }

    #[test]
    fn ring_center_is_empty_and_top_filled() {
        let b = render(Light::Yellow, Some(0.5));
        assert_eq!(px(&b, 22, 22)[3], 0);
        assert_eq!(&px(&b, 22, 3)[..3], &[0xD2, 0x99, 0x22]);
    }

    #[test]
    fn ring_left_side_is_track_at_half() {
        let b = render(Light::Yellow, Some(0.5));
        assert_eq!(&px(&b, 3, 22)[..3], &[0x55, 0x55, 0x55]);
        // right side (3 o'clock) is inside the first half → filled
        assert_eq!(&px(&b, 40, 22)[..3], &[0xD2, 0x99, 0x22]);
    }

    #[test]
    fn progress_is_quantized_to_eighths() {
        assert_eq!(render(Light::Yellow, Some(0.51)), render(Light::Yellow, Some(0.625)));
    }
}
