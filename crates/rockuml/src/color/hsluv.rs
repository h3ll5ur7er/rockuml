//! `HSLuv` conversions (`HUSLColorConverter`), used to reverse a colour's lightness perceptually.

use std::f64::consts::PI;

use super::XColor;

const M: [[f64; 3]; 3] = [
    [
        3.240_969_941_904_521,
        -1.537_383_177_570_093,
        -0.498_610_760_293,
    ],
    [
        -0.969_243_636_280_87,
        1.875_967_501_507_72,
        0.041_555_057_407_175,
    ],
    [
        0.055_630_079_696_993,
        -0.203_976_958_888_97,
        1.056_971_514_242_878,
    ],
];
const M_INVERSE: [[f64; 3]; 3] = [
    [
        0.412_390_799_265_95,
        0.357_584_339_383_87,
        0.180_480_788_401_83,
    ],
    [
        0.212_639_005_871_51,
        0.715_168_678_767_75,
        0.072_192_315_360_733,
    ],
    [
        0.019_330_818_715_591,
        0.119_194_779_794_62,
        0.950_532_152_249_66,
    ],
];
const REF_Y: f64 = 1.0;
const REF_U: f64 = 0.197_830_006_642_83;
const REF_V: f64 = 0.468_319_994_938_79;
const KAPPA: f64 = 903.296_296_2;
const EPSILON: f64 = 0.008_856_451_6;

/// `ColorUtils.reverseHsluv`. PlantUML divides channels by 256 here, not 255.
pub(super) fn reverse(color: XColor) -> XColor {
    let [hue, saturation, lightness] = rgb_to_hsluv([
        f64::from(color.red) / 256.0,
        f64::from(color.green) / 256.0,
        f64::from(color.blue) / 256.0,
    ]);
    let mut lightness = (lightness + 50.0) % 100.0;
    lightness += 0.25 * (50.0 - lightness);
    let [red, green, blue] = hsluv_to_rgb([hue, saturation, lightness]);
    XColor::rgb(to_255(red), to_255(green), to_255(blue))
}

fn to_255(value: f64) -> u8 {
    ((255.0 * value) as i32).clamp(0, 255) as u8
}

fn bounds(lightness: f64) -> Vec<[f64; 2]> {
    let sub1 = (lightness + 16.0).powf(3.0) / 1_560_896.0;
    let sub2 = if sub1 > EPSILON {
        sub1
    } else {
        lightness / KAPPA
    };
    let mut result = Vec::with_capacity(6);
    for [m1, m2, m3] in M {
        for t in [0.0, 1.0] {
            let top1 = (284_517.0 * m1 - 94_839.0 * m3) * sub2;
            let top2 = (838_422.0 * m3 + 769_860.0 * m2 + 731_718.0 * m1) * lightness * sub2
                - 769_860.0 * t * lightness;
            let bottom = (632_260.0 * m3 - 126_452.0 * m2) * sub2 + 126_452.0 * t;
            result.push([top1 / bottom, top2 / bottom]);
        }
    }
    result
}

fn max_chroma_for(lightness: f64, hue: f64) -> f64 {
    let hue_radians = hue / 360.0 * PI * 2.0;
    bounds(lightness)
        .into_iter()
        .map(|[slope, intercept]| intercept / (hue_radians.sin() - slope * hue_radians.cos()))
        .filter(|length| *length >= 0.0)
        .fold(f64::MAX, f64::min)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn from_linear(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn to_linear(c: f64) -> f64 {
    if c > 0.040_45 {
        ((c + 0.055) / (1.0 + 0.055)).powf(2.4)
    } else {
        c / 12.92
    }
}

fn rgb_to_hsluv(rgb: [f64; 3]) -> [f64; 3] {
    let linear = rgb.map(to_linear);
    let [x, y, z] = M_INVERSE.map(|row| dot(row, linear));
    let lightness = if y <= EPSILON {
        (y / REF_Y) * KAPPA
    } else {
        116.0 * (y / REF_Y).powf(1.0 / 3.0) - 16.0
    };
    let [u, v] = if lightness == 0.0 {
        [0.0, 0.0]
    } else {
        let denominator = x + 15.0 * y + 3.0 * z;
        [
            13.0 * lightness * ((4.0 * x) / denominator - REF_U),
            13.0 * lightness * ((9.0 * y) / denominator - REF_V),
        ]
    };
    let chroma = (u * u + v * v).sqrt();
    let hue = if chroma < 0.000_000_01 {
        0.0
    } else {
        let degrees = (v.atan2(u) * 180.0) / PI;
        if degrees < 0.0 {
            360.0 + degrees
        } else {
            degrees
        }
    };
    if lightness > 99.999_999_9 {
        return [hue, 0.0, 100.0];
    }
    if lightness < 0.000_000_01 {
        return [hue, 0.0, 0.0];
    }
    [
        hue,
        chroma / max_chroma_for(lightness, hue) * 100.0,
        lightness,
    ]
}

fn hsluv_to_rgb([hue, saturation, lightness]: [f64; 3]) -> [f64; 3] {
    let (lightness, chroma) = if lightness > 99.999_999_9 {
        (100.0, 0.0)
    } else if lightness < 0.000_000_01 {
        (0.0, 0.0)
    } else {
        (
            lightness,
            max_chroma_for(lightness, hue) / 100.0 * saturation,
        )
    };
    let hue_radians = hue / 360.0 * 2.0 * PI;
    let (u, v) = (hue_radians.cos() * chroma, hue_radians.sin() * chroma);
    let xyz = if lightness == 0.0 {
        [0.0, 0.0, 0.0]
    } else {
        let var_u = u / (13.0 * lightness) + REF_U;
        let var_v = v / (13.0 * lightness) + REF_V;
        let y = if lightness <= 8.0 {
            REF_Y * lightness / KAPPA
        } else {
            REF_Y * ((lightness + 16.0) / 116.0).powf(3.0)
        };
        let x = 0.0 - (9.0 * y * var_u) / ((var_u - 4.0) * var_v - var_u * var_v);
        let z = (9.0 * y - (15.0 * var_v * y) - (var_v * x)) / (3.0 * var_v);
        [x, y, z]
    };
    M.map(|row| from_linear(dot(row, xyz)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_plantuml_reverse_hsluv() {
        let cases = [
            (0x000000, 0x767676),
            (0xFFFFFF, 0x767676),
            (0x1E90FF, 0x012F5B),
            (0xFF0000, 0x530000),
            (0x808080, 0x252525),
            (0xABCDEF, 0x325773),
        ];
        for (input, expected) in cases {
            assert_eq!(
                reverse(XColor::from_rgb(input)),
                XColor::from_rgb(expected),
                "{input:06X}"
            );
        }
    }
}
