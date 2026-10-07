//! Java 2D's `AffineTransformOp` with `TYPE_BILINEAR`, which runs medialib's `mlib_ImageAffine` with
//! `MLIB_EDGE_SRC_EXTEND` (`libmlib_image` in `OpenJDK`). Pixels whose four neighbours lie inside the source are
//! interpolated in 16-bit fixed point (`mlib_ImageAffine_u8_4ch_bl`); those around the border in doubles,
//! with the border pixels repeated (`mlib_ImageAffineEdgeExtend_BL`). The four channels of a pixel are
//! interpolated independently, without premultiplying alpha.

use std::ops::RangeInclusive;

use super::PortableImage;

const MLIB_SHIFT: i32 = 16;
const MLIB_PREC: i32 = 1 << MLIB_SHIFT;
const MLIB_MASK: i32 = MLIB_PREC - 1;
const MLIB_ROUND: i32 = 1 << (MLIB_SHIFT - 1);
const HALF_PIXEL: i32 = 1 << (MLIB_SHIFT - 1);

pub(super) fn scale(
    source: &PortableImage,
    factor: f64,
    width: usize,
    height: usize,
) -> PortableImage {
    let mut target = PortableImage::new(width, height);
    let geometry = Geometry {
        source: (source.width as i32, source.height as i32),
        target: (width as i32, height as i32),
        matrix: [factor, 0.0, 0.0, 0.0, factor, 0.0],
    };
    let interior = geometry.affine_edges(Region::Interior);
    let whole = geometry.affine_edges(Region::Whole);
    affine_bl(source, &mut target, &interior);
    edge_extend_bl(source, &mut target, &interior, &whole);
    target
}

/// Which destination pixels a pass covers: those whose centre maps between the centres of the source's
/// border pixels, or anywhere on the source.
#[derive(Clone, Copy, PartialEq)]
enum Region {
    Interior,
    Whole,
}

struct Geometry {
    source: (i32, i32),
    target: (i32, i32),
    /// `a b tx c d ty`: the transform from source to destination coordinates.
    matrix: [f64; 6],
}

/// The part of the source a pass samples. `delta` moves sample positions from pixel corners to pixel
/// centres for the bilinear filter.
struct Clip {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    delta: f64,
}

impl Clip {
    /// The bilinear filter needs a pixel and its right and lower neighbour.
    fn of(region: Region, (width, height): (i32, i32)) -> Self {
        let (width, height) = (f64::from(width), f64::from(height));
        match region {
            Region::Interior => {
                let delta = -0.5;
                Self {
                    x: -delta,
                    y: -delta,
                    w: width - (1.0 + delta),
                    h: height - (1.0 + delta),
                    delta,
                }
            }
            Region::Whole => Self {
                x: 0.0,
                y: 0.0,
                w: width,
                h: height,
                delta: 0.0,
            },
        }
    }
}

/// Per destination row, the pixels a pass covers and where in the source (16.16 fixed point) the first
/// one samples (medialib's `mlib_affine_param`).
struct AffineParam {
    left_edges: Vec<i32>,
    right_edges: Vec<i32>,
    x_starts: Vec<i32>,
    y_starts: Vec<i32>,
    y_start: i32,
    y_finish: i32,
    dx: i32,
    dy: i32,
}

impl AffineParam {
    fn empty(rows: usize) -> Self {
        Self {
            left_edges: vec![0; rows],
            right_edges: vec![0; rows],
            x_starts: vec![0; rows],
            y_starts: vec![0; rows],
            y_start: 0,
            y_finish: -1,
            dx: 0,
            dy: 0,
        }
    }
}

impl Geometry {
    /// `mlib_AffineEdges` for a transform that keeps orientation (a positive determinant).
    fn affine_edges(&self, region: Region) -> AffineParam {
        let clip = Clip::of(region, self.source);
        let mut param = AffineParam::empty(self.target.1 as usize);
        if clip.x >= clip.w || clip.y >= clip.h {
            return param;
        }
        let Some((mut top, mut bot)) = self.outline(&clip, &mut param) else {
            return param;
        };
        (param.dx, param.dy) = self.sample_starts(&clip, top, bot, &mut param);
        while top <= bot && param.left_edges[top as usize] > param.right_edges[top as usize] {
            top += 1;
        }
        if top < bot {
            while param.left_edges[bot as usize] > param.right_edges[bot as usize] {
                bot -= 1;
            }
        }
        param.y_start = top;
        param.y_finish = bot;
        param
    }

    /// Scans the clip's outline in the destination: for each row from the returned first to last, the
    /// pixels whose centres lie inside it. `None` when the outline starts below the destination.
    fn outline(&self, clip: &Clip, param: &mut AffineParam) -> Option<(i32, i32)> {
        let dst_height = self.target.1;
        let [a, b, tx, c, d, ty] = self.matrix;
        let (tx, ty) = (tx - 0.5, ty - 0.5);
        let corner = |x: f64, y: f64| [x * a + y * b + tx, x * c + y * d + ty];
        let coords = [
            corner(clip.x, clip.y),
            corner(clip.w, clip.y),
            corner(clip.w, clip.h),
            corner(clip.x, clip.h),
        ];
        let at = |index: usize| coords[index & 3];

        let mut top_idx = 0;
        for i in 1..4 {
            if coords[i][1] < coords[top_idx][1] {
                top_idx = i;
            }
        }
        let d_top = coords[top_idx][1];
        let mut top = d_top as i32;
        if top >= dst_height {
            return None;
        }
        if d_top >= 0.0 {
            if d_top == f64::from(top) {
                let mut x_left = coords[top_idx][0];
                let mut x_right = coords[top_idx][0];
                for next in [at(top_idx + 1), at(top_idx + 3)] {
                    if d_top == next[1] {
                        x_left = x_left.min(next[0]);
                        x_right = x_right.max(next[0]);
                    }
                }
                param.left_edges[top as usize] = ceil_like_medialib(x_left);
                param.right_edges[top as usize] = x_right as i32;
            } else {
                top += 1;
            }
        } else {
            top = 0;
        }

        for i in 0..2 {
            if let Some((rows, mut x, slope)) =
                edge_rows(at(top_idx + 4 - i), at(top_idx + 3 - i), dst_height)
            {
                for j in rows {
                    param.left_edges[j as usize] = ceil_like_medialib(x);
                    x += slope;
                }
            }
        }
        let mut bot = -1;
        for i in 0..2 {
            if let Some((rows, mut x, slope)) =
                edge_rows(at(top_idx + i), at(top_idx + i + 1), dst_height)
            {
                bot = *rows.end();
                for j in rows {
                    param.right_edges[j as usize] = x as i32;
                    x += slope;
                }
            }
        }
        Some((top, bot))
    }

    /// Where each row's first pixel samples the source, after moving in the row's ends where their pixels
    /// sample outside the clip; returns the step from one pixel's sample to the next.
    fn sample_starts(
        &self,
        clip: &Clip,
        top: i32,
        bot: i32,
        param: &mut AffineParam,
    ) -> (i32, i32) {
        let dst_width = self.target.0;
        let [a, b, tx, c, d, ty] = self.matrix;
        let mut div = a * d - b * c;
        assert!(div > 0.0, "only scaling is ported");
        let (a2, b2, tx2) = (d, -b, -d * tx + b * ty);
        let (c2, d2, ty2) = (-c, a, c * tx - a * ty);
        let inverse = |x: f64, y: f64| (x * a2 + y * b2 + tx2, x * c2 + y * d2 + ty2);

        let (x_low, y_low) = (clip.x * div, clip.y * div);
        let (x_high, y_high) = (clip.w * div, clip.h * div);
        let outside = |(x, y): (f64, f64)| x < x_low || x >= x_high || y < y_low || y >= y_high;
        let delta = clip.delta;
        let x_cl = (clip.x + delta) as i32;
        let y_cl = (clip.y + delta) as i32;
        let w_cl = (clip.w + delta) as i32;
        let h_cl = (clip.h + delta) as i32;

        div = 1.0 / div;
        let mut sdx = (a2 * div * f64::from(MLIB_PREC)) as i32;
        let mut sdy = (c2 * div * f64::from(MLIB_PREC)) as i32;
        let fixed = |value: f64| ((value * div + delta) * f64::from(MLIB_PREC)) as i32;
        let clamped = |start: i32, low: i32, high: i32| {
            let whole = start >> MLIB_SHIFT;
            if whole < low {
                low << MLIB_SHIFT
            } else if whole >= high {
                (high << MLIB_SHIFT) - 1
            } else {
                start
            }
        };
        let nudged = |step: i32| if step > 0 { step - 1 } else { step + 1 };

        for i in top..=bot {
            let row = i as usize;
            let mut x_left = param.left_edges[row].max(0);
            let mut x_right = param.right_edges[row].min(dst_width - 1);
            let center_y = f64::from(i) + 0.5;
            let mut start = inverse(f64::from(x_left) + 0.5, center_y);
            if outside(start) {
                start = (start.0 + a2, start.1 + c2);
                x_left += 1;
                if outside(start) {
                    x_right = -1;
                }
            }
            let end = inverse(f64::from(x_right) + 0.5, center_y);
            if outside(end) {
                x_right -= 1;
                if outside((end.0 - a2, end.1 - c2)) {
                    x_right = -1;
                }
            }

            let xs = clamped(fixed(start.0), x_cl, w_cl);
            let ys = clamped(fixed(start.1), y_cl, h_cl);
            if x_right >= x_left {
                let x_e = ((x_right - x_left) * sdx + xs) >> MLIB_SHIFT;
                let y_e = ((x_right - x_left) * sdy + ys) >> MLIB_SHIFT;
                if x_e < x_cl || x_e >= w_cl {
                    sdx = nudged(sdx);
                }
                if y_e < y_cl || y_e >= h_cl {
                    sdy = nudged(sdy);
                }
            }
            param.left_edges[row] = x_left;
            param.right_edges[row] = x_right;
            param.x_starts[row] = xs;
            param.y_starts[row] = ys;
        }
        (sdx, sdy)
    }
}

/// The destination rows an edge of the source's outline crosses, where it crosses the first, and how far
/// it moves per row; `None` for a horizontal edge.
fn edge_rows(
    [x1, y1]: [f64; 2],
    [x2, y2]: [f64; 2],
    dst_height: i32,
) -> Option<(RangeInclusive<i32>, f64, f64)> {
    if y1 == y2 {
        return None;
    }
    let slope = (x2 - x1) / (y2 - y1);
    if !slope.is_finite() {
        return None;
    }
    let first = if y1 < 0.0 { 0 } else { (y1 + 1.0) as i32 };
    let last = (y2 as i32).min(dst_height - 1);
    let x = x1 + slope * (f64::from(first) - y1);
    Some((first..=last, x, slope))
}

/// The first pixel at or right of `x`.
fn ceil_like_medialib(x: f64) -> i32 {
    let t = x as i32;
    if f64::from(t) >= x { t } else { t + 1 }
}

fn channels(image: &PortableImage, x: i32, y: i32) -> [u8; 4] {
    image.get_rgb(x as usize, y as usize).to_be_bytes()
}

/// `mlib_ImageAffine_u8_4ch_bl`: the interior pixels, in fixed point.
fn affine_bl(source: &PortableImage, target: &mut PortableImage, param: &AffineParam) {
    for j in param.y_start..=param.y_finish {
        let row = j as usize;
        let (mut x, mut y) = (param.x_starts[row], param.y_starts[row]);
        for column in param.left_edges[row]..=param.right_edges[row] {
            let fdx = x & MLIB_MASK;
            let fdy = y & MLIB_MASK;
            let (x_src, y_src) = (x >> MLIB_SHIFT, y >> MLIB_SHIFT);
            let a00 = channels(source, x_src, y_src);
            let a01 = channels(source, x_src + 1, y_src);
            let a10 = channels(source, x_src, y_src + 1);
            let a11 = channels(source, x_src + 1, y_src + 1);
            let result: [u8; 4] = std::array::from_fn(|k| {
                let [a00, a01, a10, a11] = [a00[k], a01[k], a10[k], a11[k]].map(i32::from);
                let pix0 = a00 + ((fdy * (a10 - a00) + MLIB_ROUND) >> MLIB_SHIFT);
                let pix1 = a01 + ((fdy * (a11 - a01) + MLIB_ROUND) >> MLIB_SHIFT);
                (pix0 + ((fdx * (pix1 - pix0) + MLIB_ROUND) >> MLIB_SHIFT)) as u8
            });
            target.set_rgb(column as usize, row, u32::from_be_bytes(result));
            x += param.dx;
            y += param.dy;
        }
    }
}

/// `mlib_ImageAffineEdgeExtend_BL`: the pixels of the whole region the interior pass left, in doubles,
/// with the source's border pixels standing in for those beyond it.
fn edge_extend_bl(
    source: &PortableImage,
    target: &mut PortableImage,
    interior: &AffineParam,
    whole: &AffineParam,
) {
    let mut line = |row: i32, from: i32, to: i32, mut x: i32, mut y: i32| {
        for column in from..to {
            let pixel = extended_sample(source, x - HALF_PIXEL, y - HALF_PIXEL);
            target.set_rgb(column as usize, row as usize, pixel);
            x += whole.dx;
            y += whole.dy;
        }
    };
    for i in whole.y_start..=whole.y_finish {
        let row = i as usize;
        let left_e = whole.left_edges[row];
        let right_e = whole.right_edges[row] + 1;
        let (x_start, y_start) = (whole.x_starts[row], whole.y_starts[row]);
        if i < interior.y_start || i > interior.y_finish {
            line(i, left_e, right_e, x_start, y_start);
            continue;
        }
        let left = interior.left_edges[row];
        let mut right = interior.right_edges[row] + 1;
        if left < right {
            line(i, left_e, left, x_start, y_start);
        } else {
            right = left_e;
        }
        let skipped = right - left_e;
        line(
            i,
            right,
            right_e,
            x_start + whole.dx * skipped,
            y_start + whole.dy * skipped,
        );
    }
}

/// The bilinear sample at fixed-point `(x, y)`, measured from the first pixel's centre.
fn extended_sample(source: &PortableImage, x: i32, y: i32) -> u32 {
    const SCALE: f64 = 1.0 / MLIB_PREC as f64;
    let (src_width, src_height) = (source.width as i32, source.height as i32);
    let t = f64::from(x & MLIB_MASK) * SCALE;
    let u = f64::from(y & MLIB_MASK) * SCALE;
    let (x_src, x_next) = extended(x >> MLIB_SHIFT, src_width);
    let (y_src, y_next) = extended(y >> MLIB_SHIFT, src_height);
    let a00 = channels(source, x_src, y_src);
    let a01 = channels(source, x_next, y_src);
    let a10 = channels(source, x_src, y_next);
    let a11 = channels(source, x_next, y_next);
    let result: [u8; 4] = std::array::from_fn(|k| {
        let [a00, a01, a10, a11] = [a00[k], a01[k], a10[k], a11[k]].map(f64::from);
        let pix0 = (a00 * (1.0 - t) + a01 * t) * (1.0 - u) + (a10 * (1.0 - t) + a11 * t) * u;
        pix0 as u8
    });
    u32::from_be_bytes(result)
}

/// A pixel index before the first pixel or at the last stands for that pixel, and so does its neighbour.
fn extended(index: i32, size: i32) -> (i32, i32) {
    if index < 0 {
        (index + 1, index + 1)
    } else if index + 1 >= size {
        (index.min(size - 1), index.min(size - 1))
    } else {
        (index, index + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: usize, height: usize, pixels: &[u32]) -> PortableImage {
        PortableImage {
            width,
            height,
            pixels: pixels.to_vec(),
        }
    }

    // Expected pixels from Java 21's `AffineTransformOp`. A port check against 593 random images and
    // scales, ARGB and RGB, matched them all.

    #[test]
    fn scales_up_slightly_like_java2d() {
        let source = image(
            3,
            2,
            &[
                0xbb1ad573, 0x19b89cd8, 0x68fb0e6f, 0x684df992, 0x352cccfc, 0x0946b8f0,
            ],
        );
        assert_eq!(
            source.scale(14.0 / 13.0).pixels,
            [
                0xbb1ad573, 0x2aa7a2cd, 0x59ef2781, 0x7047f58e, 0x393ccced, 0x1954ace6
            ]
        );
    }

    #[test]
    fn interpolates_between_border_pixels_like_java2d() {
        let source = image(2, 2, &[0xbb0f1799, 0xa3773418, 0xbfc9945a, 0x02770b39]);
        assert_eq!(
            source.scale(1.5).pixels,
            [
                0xbb0f1799, 0xaf422558, 0xa3773418, 0xbd6c5579, 0x88723b52, 0x52771f28, 0xbfc9945a,
                0x60a04f49, 0x02770b39
            ]
        );
    }

    #[test]
    fn scales_down_like_java2d() {
        let source = image(
            4,
            2,
            &[
                0xbaf1bcf8, 0xfbd3ae38, 0x98cde324, 0x03ddb75b, 0x3325e228, 0x206435ed, 0xcf057a26,
                0x73af2bd6,
            ],
        );
        assert_eq!(source.scale(0.5).pixels, [0x8394a192, 0x7898905f]);
    }
}
