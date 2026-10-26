//! Color conversions for the custom color picker.

/// `0xRRGGBB` from hue (degrees, 0..360), saturation and value (0..1).
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> u32 {
    let h = h.rem_euclid(360.0) / 60.0;
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let byte = |channel: f32| ((channel + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

/// Hue (degrees), saturation and value (0..1) of `0xRRGGBB`. A grey has hue 0.
pub fn rgb_to_hsv(rgb: u32) -> (f32, f32, f32) {
    let channel = |shift: u32| ((rgb >> shift) & 0xff) as f32 / 255.0;
    let (r, g, b) = (channel(16), channel(8), channel(0));
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue, if max == 0.0 { 0.0 } else { delta / max }, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for rgb in [0xf08a4b, 0x000000, 0xffffff, 0x5b9cf0, 0x123456] {
            let (h, s, v) = rgb_to_hsv(rgb);
            let back = hsv_to_rgb(h, s, v);
            for shift in [16, 8, 0] {
                let (a, b) = ((rgb >> shift) & 0xff, (back >> shift) & 0xff);
                assert!(a.abs_diff(b) <= 1, "{rgb:06x} -> {back:06x}");
            }
        }
    }

    #[test]
    fn primaries() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), 0xff0000);
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), 0x00ff00);
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), 0x0000ff);
        assert_eq!(hsv_to_rgb(360.0, 1.0, 1.0), 0xff0000);
        assert_eq!(hsv_to_rgb(200.0, 0.0, 1.0), 0xffffff);
        assert_eq!(rgb_to_hsv(0x808080), (0.0, 0.0, 128.0 / 255.0));
    }
}
