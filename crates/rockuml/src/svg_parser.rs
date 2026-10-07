//! PlantUML's small SVG reader (`SvgNanoParser`): draws the paths, circles, ellipses and texts of an SVG picture,
//! in its groups and transforms, as PlantUML shapes. Emoji and SVG sprites are drawn with it.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::color::HColor;
use crate::emoji::{ColorResolver, UGraphicWithScale};
use crate::java;
use crate::klimt::TextBlock;
use crate::klimt::affine::XAffineTransform;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::shape::{UEllipse, UImageSvg, USegment, UShape, UText};
use crate::klimt::sprite::Sprite;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::openiconic::SvgPath;
use crate::pattern::java_regex;

pub(crate) struct SvgNanoParser {
    svg: Cow<'static, str>,
    /// Where the elements PlantUML draws, and the ends of groups, are in `svg`, in document order.
    data: Vec<Range<usize>>,
}

impl SvgNanoParser {
    pub(crate) fn new(svg: impl Into<Cow<'static, str>>) -> Self {
        static TEXT_OR_DRAW: LazyLock<Regex> = LazyLock::new(|| {
            regex(r"(<text .*?</text>)|(<(svg|path|g|circle|ellipse)[^<>]*>)|(</[^<>]*>)")
        });
        const DRAWN: [&str; 7] = [
            "<path",
            "<g ",
            "<g>",
            "</g>",
            "<circle ",
            "<ellipse ",
            "<text ",
        ];
        let svg = svg.into();
        let data = TEXT_OR_DRAW
            .find_iter(&svg)
            .filter(|element| {
                DRAWN
                    .iter()
                    .any(|start| element.as_str().starts_with(start))
            })
            .map(|element| element.range())
            .collect();
        Self { svg, data }
    }

    fn data(&self) -> impl Iterator<Item = &str> {
        self.data.iter().map(|range| &self.svg[range.clone()])
    }

    /// Draws the picture at `scale`. Shapes without a colour take the forced colour, else the font colour; a
    /// forced colour also turns the picture's own colours into its shades.
    pub(crate) fn draw_u(
        &self,
        ug: &UGraphic,
        scale: f64,
        font_color: Option<HColor>,
        forced_color: Option<HColor>,
    ) {
        let color_resolver = ColorResolver::new(font_color, forced_color, self.min_gray_level());
        let mut ugs = UGraphicWithScale::new(ug, &color_resolver, scale);
        let mut stack = Vec::new();
        let mut stack_g = Vec::new();
        for s in self.data() {
            if s.starts_with("<path ") {
                draw_path(&ugs, s, &stack_g);
            } else if s.starts_with("</g>") {
                if let Some(enclosing) = stack.pop() {
                    ugs = enclosing;
                }
                stack_g.pop();
            } else if s.starts_with("<g>") {
                stack.push(ugs.clone());
                stack_g.push(s);
            } else if s.starts_with("<g ") {
                stack.push(ugs.clone());
                stack_g.push(s);
                ugs = apply_transform(&apply_fill_and_stroke(&ugs, s, &stack_g), s);
            } else if s.starts_with("<circle ") {
                draw_circle(&ugs, s, &stack_g);
            } else if s.starts_with("<ellipse ") {
                draw_ellipse(&ugs, s, &stack_g);
            } else if s.starts_with("<text ") {
                draw_text(&ugs, s, &stack_g);
            }
        }
    }

    /// The darkest gray among the colours the picture names.
    fn min_gray_level(&self) -> i32 {
        const COLORED: [&str; 4] = ["<path ", "<g ", "<circle ", "<ellipse "];
        self.data()
            .filter(|s| COLORED.iter().any(|element| s.contains(element)))
            .flat_map(|s| [extract(&DATA_STROKE, s), get_fill_string(s, &[])])
            .flatten()
            .filter_map(
                |color| match HColor::parse_or_white(color).as_monochrome() {
                    HColor::Simple(gray) => Some(i32::from(gray.green)),
                    _ => None,
                },
            )
            .fold(999, i32::min)
    }
}

/// An SVG sprite, measured by its `viewBox` or else its size attributes.
impl Sprite for SvgNanoParser {
    fn as_text_block(
        &self,
        font_color: &HColor,
        forced_color: Option<&HColor>,
        scale: f64,
    ) -> Box<dyn TextBlock + '_> {
        let data = UImageSvg::new(self.svg.to_string(), scale);
        Box::new(SpriteBlock {
            parser: self,
            font_color: font_color.clone(),
            forced_color: forced_color.cloned(),
            scale,
            dimension: XDimension2D::new(data.width(), data.height()),
        })
    }
}

struct SpriteBlock<'a> {
    parser: &'a SvgNanoParser,
    font_color: HColor,
    forced_color: Option<HColor>,
    scale: f64,
    dimension: XDimension2D,
}

impl TextBlock for SpriteBlock<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        self.dimension
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.parser.draw_u(
            ug,
            self.scale,
            Some(self.font_color.clone()),
            self.forced_color.clone(),
        );
    }
}

fn regex(pattern: &str) -> Regex {
    java_regex(pattern, false)
}

macro_rules! attribute {
    ($name:ident, $attribute:literal) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| regex(concat!($attribute, r#"="([^"]+)""#)));
    };
}

attribute!(DATA_CX, "cx");
attribute!(DATA_CY, "cy");
attribute!(DATA_FILL, "fill");
attribute!(DATA_FONT_FAMILY, "font-family");
attribute!(DATA_FONT_SIZE, "font-size");
attribute!(DATA_R, "r");
attribute!(DATA_RX, "rx");
attribute!(DATA_RY, "ry");
attribute!(DATA_STROKE, "stroke");
attribute!(DATA_STROKE_WIDTH, "stroke-width");
attribute!(DATA_STYLE, "style");
attribute!(DATA_TRANSFORM, "transform");
attribute!(DATA_X, "x");
attribute!(DATA_Y, "y");

macro_rules! style_property {
    ($name:ident, $property:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| regex(concat!($property, r#":([^;"]+)"#)));
    };
}

style_property!(STYLE_FILL, "fill");
style_property!(STYLE_FONT_SIZE, "font-size");
style_property!(STYLE_FONT_FAMILY, "font-family");

/// The first group of the first match.
fn extract<'s>(pattern: &Regex, s: &'s str) -> Option<&'s str> {
    pattern
        .captures(s)
        .and_then(|captures| captures.get(1))
        .map(|group| group.as_str())
}

/// Malformed numbers, which stop PlantUML, count as zero.
fn parse_double(text: &str) -> f64 {
    java::trim(text).parse().unwrap_or_default()
}

/// The element's attribute or style property, else that of the innermost group that has one.
fn inherited<'s>(
    s: &'s str,
    stack_g: &[&'s str],
    attribute: &Regex,
    style_property: &Regex,
) -> Option<&'s str> {
    let own = |s: &'s str| {
        extract(attribute, s)
            .or_else(|| extract(&DATA_STYLE, s).and_then(|style| extract(style_property, style)))
    };
    own(s).or_else(|| stack_g.iter().rev().find_map(|g| own(g)))
}

fn get_fill_string<'s>(s: &'s str, stack_g: &[&'s str]) -> Option<&'s str> {
    inherited(s, stack_g, &DATA_FILL, &STYLE_FILL)
}

fn get_text_font_family<'s>(s: &'s str, stack_g: &[&'s str]) -> Option<&'s str> {
    inherited(s, stack_g, &DATA_FONT_FAMILY, &STYLE_FONT_FAMILY)
}

/// In points or pixels, 14 when not given.
fn get_text_font_size(s: &str) -> i32 {
    static UNIT: LazyLock<Regex> = LazyLock::new(|| regex(r"^(\d+)p[tx]$"));
    let font_size = extract(&DATA_FONT_SIZE, s)
        .or_else(|| extract(&DATA_STYLE, s).and_then(|style| extract(&STYLE_FONT_SIZE, style)));
    font_size.map_or(14, |font_size| {
        let digits = UNIT
            .captures(font_size)
            .map_or(font_size, |captures| captures.get(1).unwrap().as_str());
        digits.parse().unwrap_or_default()
    })
}

fn apply_fill_and_stroke<'a>(
    ugs: &UGraphicWithScale<'a>,
    s: &str,
    stack_g: &[&str],
) -> UGraphicWithScale<'a> {
    let fill_string = get_fill_string(s, stack_g);
    let stroke_string = extract(&DATA_STROKE, s);
    let mut ugs = ugs.clone();
    if let Some(stroke_width) = extract(&DATA_STROKE_WIDTH, s) {
        let thickness = ugs.initial_scale() * parse_double(stroke_width);
        ugs = ugs.with_stroke(UStroke::with_thickness(thickness));
    }
    if let Some(stroke_string) = stroke_string {
        ugs = ugs.with_color(ugs.true_color(stroke_string));
        if fill_string.is_none() {
            return ugs.with_backcolor(ugs.default_color());
        }
    }
    if fill_string == Some("none") {
        return ugs.with_backcolor(HColor::NONE);
    }
    let fill = fill_string.map_or_else(|| ugs.default_color(), |fill| ugs.true_color(fill));
    if stroke_string.is_none() {
        ugs = ugs.with_color(fill.clone());
    }
    ugs.with_backcolor(fill)
}

/// A rotation or a matrix replaces any other transform of the element; otherwise it translates, then scales.
fn apply_transform<'a>(ugs: &UGraphicWithScale<'a>, s: &str) -> UGraphicWithScale<'a> {
    static ROTATE: LazyLock<Regex> =
        LazyLock::new(|| regex(r"rotate\(([-.0-9]+)[ ,]+([-.0-9]+)[ ,]+([-.0-9]+)\)"));
    static MATRIX: LazyLock<Regex> = LazyLock::new(|| {
        regex(
            r"matrix\(([-.0-9]+)[ ,]+([-.0-9]+)[ ,]+([-.0-9]+)[ ,]+([-.0-9]+)[ ,]+([-.0-9]+)[ ,]+([-.0-9]+)\)",
        )
    });
    let Some(transform) = extract(&DATA_TRANSFORM, s) else {
        return ugs.clone();
    };
    let arguments = |pattern: &Regex| {
        pattern.captures(transform).map(|captures| {
            captures
                .iter()
                .skip(1)
                .map(|group| parse_double(group.unwrap().as_str()))
                .collect::<Vec<_>>()
        })
    };
    if transform.contains("rotate(") {
        return arguments(&ROTATE).map_or_else(
            || ugs.clone(),
            |rotate| ugs.apply_rotate(rotate[0], rotate[1], rotate[2]),
        );
    }
    if transform.contains("matrix(") {
        return arguments(&MATRIX).map_or_else(
            || ugs.clone(),
            |m| ugs.apply_matrix(&XAffineTransform::new(m[0], m[1], m[2], m[3], m[4], m[5])),
        );
    }
    let (scale_x, scale_y) = get_scale(transform);
    let translate = get_translate(transform);
    ugs.apply_translate(translate.dx, translate.dy)
        .apply_scale(scale_x, scale_y)
}

/// `translate(x)` moves along both axes in PlantUML.
fn get_translate(transform: &str) -> UTranslate {
    static TRANSLATE1: LazyLock<Regex> =
        LazyLock::new(|| regex(r"translate\(([-.0-9]+)[ ,]+([-.0-9]+)\)"));
    static TRANSLATE2: LazyLock<Regex> = LazyLock::new(|| regex(r"translate\(([-.0-9]+)\)"));
    if let Some(captures) = TRANSLATE1.captures(transform) {
        return UTranslate::new(parse_double(&captures[1]), parse_double(&captures[2]));
    }
    TRANSLATE2
        .captures(transform)
        .map_or(UTranslate::default(), |captures| {
            let offset = parse_double(&captures[1]);
            UTranslate::new(offset, offset)
        })
}

fn get_scale(transform: &str) -> (f64, f64) {
    static SCALE1: LazyLock<Regex> = LazyLock::new(|| regex(r"scale\(([-.0-9]+)\)"));
    static SCALE2: LazyLock<Regex> = LazyLock::new(|| regex(r"scale\(([-.0-9]+)[ ,]+([-.0-9]+)\)"));
    if let Some(captures) = SCALE1.captures(transform) {
        let scale = parse_double(&captures[1]);
        return (scale, scale);
    }
    SCALE2.captures(transform).map_or((1.0, 1.0), |captures| {
        (parse_double(&captures[1]), parse_double(&captures[2]))
    })
}

/// PlantUML reads the data of a `d="..."` attribute only, and fails on a path without one.
fn draw_path(ugs: &UGraphicWithScale, s: &str, stack_g: &[&str]) {
    // An `id` attribute would be read as the path data.
    let s = s.replace("id=\"", "ID=\"");
    let Some(data) = s
        .split_once("d=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(data, _)| data)
    else {
        return;
    };
    let ugs = apply_transform(&apply_fill_and_stroke(ugs, &s, stack_g), &s);
    let path = SvgPath::new(data, UTranslate::default()).to_upath_affine(ugs.affine_transform());
    if path
        .iter()
        .any(|segment| !matches!(segment, USegment::MoveTo(..)))
    {
        ugs.draw(&UShape::Path(path));
    }
}

/// Circles only take the scale and translation of the transform.
fn draw_circle(ugs: &UGraphicWithScale, s: &str, stack_g: &[&str]) {
    let ugs = apply_transform(&apply_fill_and_stroke(ugs, s, stack_g), s);
    let at = ugs.affine_transform();
    let attribute = |pattern| extract(pattern, s).map_or(0.0, parse_double);
    let cx = attribute(&DATA_CX) * at.scale_x();
    let cy = attribute(&DATA_CY) * at.scale_y();
    let rx = attribute(&DATA_R) * at.scale_x();
    let ry = attribute(&DATA_R) * at.scale_y();
    ugs.ug()
        .translated(at.translate_x() + cx - rx, at.translate_y() + cy - ry)
        .draw(&UShape::Ellipse(UEllipse::new(rx * 2.0, ry * 2.0)));
}

/// Ellipses become four arcs, which turn with the transform's rotation.
fn draw_ellipse(ugs: &UGraphicWithScale, s: &str, stack_g: &[&str]) {
    let ugs = apply_transform(&apply_fill_and_stroke(ugs, s, stack_g), s);
    let attribute = |pattern| extract(pattern, s).map_or(0.0, parse_double);
    let (cx, cy) = (attribute(&DATA_CX), attribute(&DATA_CY));
    let (rx, ry) = (attribute(&DATA_RX), attribute(&DATA_RY));
    let arc = |end| USegment::ArcTo {
        radius: (rx, ry),
        x_axis_rotation: 0.0,
        large_arc: false,
        sweep: true,
        end,
    };
    let mut path = vec![
        USegment::MoveTo(0.0, ry),
        arc((rx, 0.0)),
        arc((2.0 * rx, ry)),
        arc((rx, 2.0 * ry)),
        arc((0.0, ry)),
    ];
    let (dx, dy) = (cx - rx, cy - ry);
    if dx != 0.0 || dy != 0.0 {
        for segment in &mut path {
            *segment = segment.translate(dx, dy);
        }
    }
    let path = path
        .into_iter()
        .map(|segment| {
            affine(
                segment,
                ugs.affine_transform(),
                ugs.angle(),
                ugs.initial_scale(),
            )
        })
        .collect();
    ugs.draw(&UShape::Path(path));
}

/// PlantUML's `USegment.affine`: arcs turn by `angle` and grow by `scale` whatever the transform.
fn affine(segment: USegment, at: &XAffineTransform, angle: f64, scale: f64) -> USegment {
    match segment {
        USegment::MoveTo(x, y) => {
            let (x, y) = at.transform((x, y));
            USegment::MoveTo(x, y)
        }
        USegment::ArcTo {
            radius: (rx, ry),
            x_axis_rotation,
            large_arc,
            sweep,
            end,
        } => USegment::ArcTo {
            radius: (rx * scale, ry * scale),
            x_axis_rotation: x_axis_rotation + angle,
            large_arc,
            sweep,
            end: at.transform(end),
        },
        USegment::LineTo(..) | USegment::CubicTo { .. } => {
            unreachable!("ellipses are drawn with arcs")
        }
    }
}

/// Texts keep their own colour whatever the forced one.
fn draw_text(ugs: &UGraphicWithScale, s: &str, stack_g: &[&str]) {
    static TEXT: LazyLock<Regex> = LazyLock::new(|| regex(r"<text[^<>]*>(.*?)</text>"));
    let ugs = apply_transform(&apply_fill_and_stroke(ugs, s, stack_g), s);
    let at = ugs.affine_transform();
    let attribute = |pattern| extract(pattern, s).map_or(0.0, parse_double);
    let x = attribute(&DATA_X) * at.scale_x() + at.translate_x();
    let y = attribute(&DATA_Y) * at.scale_y() + at.translate_y();
    let font_size = (f64::from(get_text_font_size(s)) * ugs.initial_scale()) as i32;
    let Some(text) = extract(&TEXT, s) else {
        return;
    };
    let color = get_fill_string(s, stack_g).map_or(HColor::WHITE, HColor::parse_or_white);
    let family = get_text_font_family(s, stack_g).unwrap_or("SansSerif");
    let font = FontConfiguration::new(UFont::new(family, UFontFace::NORMAL, font_size), color, 8);
    ugs.ug()
        .translated(x, y)
        .draw(&UShape::Text(UText::new(text, font)));
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::klimt::debug::StringBounderDebug;
    use crate::klimt::ugraphic::{UGraphicBackend, UParam};

    #[derive(Default)]
    struct Recorder {
        shapes: Vec<(UShape, UTranslate, UParam)>,
    }

    impl UGraphicBackend for Recorder {
        fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
            self.shapes.push((shape.clone(), at, param.clone()));
        }
    }

    fn draw(
        svg: &str,
        scale: f64,
        forced_color: Option<&str>,
    ) -> Vec<(UShape, UTranslate, UParam)> {
        let recorder = Rc::new(RefCell::new(Recorder::default()));
        let ug = UGraphic::new(recorder.clone(), Rc::new(StringBounderDebug), HColor::WHITE);
        SvgNanoParser::new(svg.to_owned()).draw_u(
            &ug,
            scale,
            None,
            forced_color.map(HColor::parse_or_white),
        );
        recorder.take().shapes
    }

    fn color(text: &str) -> HColor {
        HColor::parse_or_white(text)
    }

    #[test]
    fn shapes_inherit_the_fill_of_their_groups() {
        let shapes = draw(
            r##"<g fill="#123456"><circle cx="2" cy="3" r="1"/><g><path fill="#FFF" d="M0 0h1"/></g></g><circle cx="1" cy="1" r="1"/>"##,
            2.0,
            None,
        );
        assert_eq!(shapes.len(), 3);
        let (shape, at, param) = &shapes[0];
        assert_eq!(*shape, UShape::Ellipse(UEllipse::new(4.0, 4.0)));
        assert_eq!(*at, UTranslate::new(2.0, 4.0));
        assert_eq!(
            (&param.color, &param.backcolor),
            (&color("#123456"), &color("#123456"))
        );
        assert_eq!(
            shapes[1].0,
            UShape::Path(vec![USegment::MoveTo(0.0, 0.0), USegment::LineTo(2.0, 0.0)])
        );
        assert_eq!(shapes[1].2.backcolor, color("#FFF"));
        assert_eq!(shapes[2].2.backcolor, HColor::BLACK);
    }

    #[test]
    fn a_stroke_without_fill_fills_with_the_default_colour() {
        let shapes = draw(
            r##"<path stroke="#F00" stroke-width="2" d="M0 0h1"/>"##,
            0.5,
            None,
        );
        let param = &shapes[0].2;
        assert_eq!(
            (&param.color, &param.backcolor),
            (&color("#F00"), &HColor::BLACK)
        );
        assert_eq!(param.stroke, UStroke::with_thickness(1.0));
    }

    #[test]
    fn groups_translate_then_scale_the_translation_too() {
        let shapes = draw(
            r#"<g transform="translate(1 2) scale(3)"><path d="M1 1"/><path d="M1 1h1"/></g><g transform="translate(5)"><path d="M0 0h1"/></g>"#,
            1.0,
            None,
        );
        assert_eq!(shapes.len(), 2, "a path that only moves draws nothing");
        assert_eq!(
            shapes[0].0,
            UShape::Path(vec![USegment::MoveTo(6.0, 9.0), USegment::LineTo(9.0, 9.0)])
        );
        assert_eq!(
            shapes[1].0,
            UShape::Path(vec![USegment::MoveTo(5.0, 5.0), USegment::LineTo(6.0, 5.0)])
        );
    }

    #[test]
    fn paths_without_double_quoted_data_draw_nothing() {
        for svg in [
            r#"<path fill="red"/>"#,
            "<path d='M0 0 h5'/>",
            r#"<path fill="red" d="M0 0 h5>"#,
        ] {
            assert!(draw(svg, 1.0, None).is_empty(), "{svg}");
        }
    }

    #[test]
    fn texts_are_placed_and_sized_with_the_scale() {
        let shapes = draw(
            r#"<text x="2" y="4" font-size="10px" style="font-family:Serif" fill="red">Hi</text>"#,
            2.0,
            None,
        );
        let (shape, at, _) = &shapes[0];
        assert_eq!(*at, UTranslate::new(4.0, 8.0));
        let UShape::Text(text) = shape else {
            panic!("a text")
        };
        assert_eq!(text.text, "Hi");
        assert_eq!(text.font.font(), UFont::new("Serif", UFontFace::NORMAL, 20));
        assert_eq!(*text.font.color(), color("red"));
    }

    #[test]
    fn a_forced_colour_shades_from_the_darkest_colour() {
        let shapes = draw(
            r##"<path fill="#664500" d="M0 0h1"/><path fill="#FFCC4D" d="M0 0h1"/>"##,
            1.0,
            Some("orange"),
        );
        assert_eq!(shapes[0].2.backcolor, color("#FEA400"));
        assert_eq!(shapes[1].2.backcolor, color("#FED4B3"));
    }

    #[test]
    fn a_sprite_measures_its_view_box_rounded_up_else_its_size_attributes() {
        let dimension = |svg: &str, scale| {
            SvgNanoParser::new(svg.to_owned())
                .as_text_block(&HColor::BLACK, None, scale)
                .calculate_dimension(&StringBounderDebug)
        };
        assert_eq!(
            dimension(
                r#"<svg width="19.995mm" viewBox="0 0 19.995 19.2"></svg>"#,
                2.0
            ),
            XDimension2D::new(40.0, 40.0)
        );
        assert_eq!(
            dimension(
                r#"<svg stroke-width="2" width="24" height="12"></svg>"#,
                0.5
            ),
            XDimension2D::new(12.0, 6.0)
        );
    }
}
