use crate::model::Light;
use std::f32::consts::TAU;

/// 22pt menu-bar icon at 2x.
pub const ICON_PX: u32 = 44;

const CENTER: f32 = ICON_PX as f32 / 2.0;
const DISC_R: f32 = 18.0;
const RING_OUTER: f32 = 20.0;
const RING_INNER: f32 = 14.0;
const TRACK: [u8; 3] = [0x55, 0x55, 0x55];
const CONNECTOR: [u8; 3] = [0x8B, 0x94, 0x9E];

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
                    (
                        coverage(r, RING_INNER, RING_OUTER),
                        if angle < f { color } else { TRACK },
                    )
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

/// The app logo as a tray glyph for the idle state: two green steps, a yellow one and a
/// red branch, joined by grey connectors.
pub fn render_idle() -> Vec<u8> {
    const NODES: [(f32, f32, Light); 4] = [
        (7.5, 14.5, Light::Green),
        (22.5, 14.5, Light::Green),
        (37.5, 14.5, Light::Yellow),
        (22.5, 32.5, Light::Red),
    ];
    const NODE_R: f32 = 5.5;
    const HALF_LINE: f32 = 1.5;
    let mut buf = vec![0u8; (ICON_PX * ICON_PX * 4) as usize];
    for y in 0..ICON_PX {
        for x in 0..ICON_PX {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let (node, node_color) = NODES
                .iter()
                .map(|(cx, cy, light)| {
                    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                    (coverage(d, -1.0, NODE_R), rgb(*light))
                })
                .fold((0.0, CONNECTOR), |acc, n| if n.0 > acc.0 { n } else { acc });
            let row = if (7.5..=37.5).contains(&px) {
                coverage((py - 14.5).abs(), -1.0, HALF_LINE)
            } else {
                0.0
            };
            let branch = if (14.5..=32.5).contains(&py) {
                coverage((px - 22.5).abs(), -1.0, HALF_LINE)
            } else {
                0.0
            };
            let alpha = node.max(row).max(branch);
            if alpha > 0.0 {
                let i = ((y * ICON_PX + x) * 4) as usize;
                let color = if node > 0.0 { node_color } else { CONNECTOR };
                buf[i..i + 3].copy_from_slice(&color);
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
        assert_eq!(
            render(Light::Green, None).len(),
            (ICON_PX * ICON_PX * 4) as usize
        );
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
        assert_eq!(
            render(Light::Yellow, Some(0.51)),
            render(Light::Yellow, Some(0.625))
        );
    }

    #[test]
    fn idle_glyph_has_steps_and_branch() {
        let b = render_idle();
        assert_eq!(b.len(), (ICON_PX * ICON_PX * 4) as usize);
        // the three steps on the top row and the branch below the middle one
        for (x, y) in [(8, 15), (22, 15), (36, 15), (22, 33)] {
            assert_eq!(px(&b, x, y)[3], 255, "step at {x},{y}");
        }
        // the connector between steps
        assert_eq!(px(&b, 15, 15)[3], 255);
        assert_eq!(px(&b, 0, 0)[3], 0);
        assert_eq!(px(&b, 36, 33)[3], 0);
    }

    #[test]
    fn idle_glyph_uses_the_logo_colors() {
        let b = render_idle();
        assert_eq!(&px(&b, 7, 14)[..3], &rgb(Light::Green));
        assert_eq!(&px(&b, 22, 14)[..3], &rgb(Light::Green));
        assert_eq!(&px(&b, 37, 14)[..3], &rgb(Light::Yellow));
        assert_eq!(&px(&b, 22, 32)[..3], &rgb(Light::Red));
        // connectors stay neutral grey
        assert_eq!(&px(&b, 15, 14)[..3], &[0x8B, 0x94, 0x9E]);
    }
}
