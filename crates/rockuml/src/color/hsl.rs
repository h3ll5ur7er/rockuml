//! HSL conversions in single precision, step for step as PlantUML's `HSLColor` computes them.

use super::XColor;

/// Hue in degrees, saturation and luminance in percent.
pub fn from_rgb(color: XColor) -> [f32; 3] {
    let red = f32::from(color.red) / 255.0;
    let green = f32::from(color.green) / 255.0;
    let blue = f32::from(color.blue) / 255.0;
    let min = red.min(green.min(blue));
    let max = red.max(green.max(blue));
    let hue = if max == min {
        0.0
    } else if max == red {
        ((60.0 * (green - blue) / (max - min)) + 360.0) % 360.0
    } else if max == green {
        (60.0 * (blue - red) / (max - min)) + 120.0
    } else {
        (60.0 * (red - green) / (max - min)) + 240.0
    };
    #[allow(
        clippy::manual_midpoint,
        reason = "same rounding as Java's (max + min) / 2"
    )]
    let luminance = (max + min) / 2.0;
    let saturation = if max == min {
        0.0
    } else if luminance <= 0.5 {
        (max - min) / (max + min)
    } else {
        (max - min) / (2.0 - max - min)
    };
    [hue, saturation * 100.0, luminance * 100.0]
}

pub fn to_rgb(hue: f32, saturation: f32, luminance: f32, alpha: f32) -> XColor {
    let saturation = saturation.clamp(0.0, 100.0) / 100.0;
    let luminance = luminance.clamp(0.0, 100.0) / 100.0;
    let alpha = alpha.clamp(0.0, 1.0);
    let hue = (hue % 360.0) / 360.0;
    let q = if luminance < 0.5 {
        luminance * (1.0 + saturation)
    } else {
        (luminance + saturation) - (saturation * luminance)
    };
    let p = 2.0 * luminance - q;
    let red = hue_to_rgb(p, q, hue + (1.0 / 3.0)).clamp(0.0, 1.0);
    let green = hue_to_rgb(p, q, hue).clamp(0.0, 1.0);
    let blue = hue_to_rgb(p, q, hue - (1.0 / 3.0)).clamp(0.0, 1.0);
    XColor {
        red: to_byte(red),
        green: to_byte(green),
        blue: to_byte(blue),
        alpha: to_byte(alpha),
    }
}

fn hue_to_rgb(p: f32, q: f32, hue: f32) -> f32 {
    let mut hue = hue;
    if hue < 0.0 {
        hue += 1.0;
    }
    if hue > 1.0 {
        hue -= 1.0;
    }
    if 6.0 * hue < 1.0 {
        p + ((q - p) * 6.0 * hue)
    } else if 2.0 * hue < 1.0 {
        q
    } else if 3.0 * hue < 2.0 {
        p + ((q - p) * 6.0 * ((2.0 / 3.0) - hue))
    } else {
        p
    }
}

/// `Math.round(x * 255f)` for x in 0..=1.
fn to_byte(component: f32) -> u8 {
    (component * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_colours_round_trip() {
        for color in [
            XColor::rgb(255, 0, 0),
            XColor::rgb(0, 128, 0),
            XColor::rgb(30, 144, 255),
        ] {
            let [hue, saturation, luminance] = from_rgb(color);
            assert_eq!(to_rgb(hue, saturation, luminance, 1.0), color);
        }
    }

    #[test]
    fn grays_have_no_hue_or_saturation() {
        assert_eq!(from_rgb(XColor::rgb(128, 128, 128))[..2], [0.0, 0.0]);
    }
}
