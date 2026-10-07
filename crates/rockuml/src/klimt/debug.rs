//! PlantUML's `debug` output format: a plain-text listing of every drawn shape, with text measured by a
//! seeded pseudo-random metric instead of real fonts so that the output is the same everywhere.

use super::clip::UClip;
use super::font::{StringBounder, UFont};
use super::geom::{UTranslate, XDimension2D};
use super::shape::{UEllipse, URectangle, USegment, UShape, UText};
use super::ugraphic::{UGraphicBackend, UParam};
use crate::color::HColor;
use crate::java::{self, Random};

pub(crate) struct StringBounderDebug;

impl StringBounder for StringBounderDebug {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> XDimension2D {
        let factor = 0.8 + 0.5 * Random::new(java::string_seed(text)).next_double();
        let size = font.size_2d();
        let utf16_length = text.encode_utf16().count() as f64;
        XDimension2D::new(size * utf16_length * factor, size)
    }
}

/// What the debug document says about the image before listing its shapes.
pub(crate) struct DebugHeader {
    pub dimension: XDimension2D,
    pub scale_factor: f64,
    pub seed: i64,
    pub svg_link_target: Option<String>,
    pub hover_path_color_rgb: Option<String>,
    pub preserve_aspect_ratio: String,
}

pub(crate) struct UGraphicDebug {
    lines: Vec<String>,
    /// PlantUML stamps shapes it cannot describe with the time, in `java.util.Date` format.
    render_date: String,
}

impl UGraphicDebug {
    pub(crate) fn new(render_date: String) -> Self {
        Self {
            lines: Vec::new(),
            render_date,
        }
    }

    /// The document, with lines ending in `\n` on every platform as in PlantUML.
    pub(crate) fn document(&self, header: &DebugHeader) -> String {
        let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| "null".to_owned());
        let header_lines = [
            "DPI: 96".to_owned(),
            format!(
                "dimension: {}",
                point(header.dimension.width, header.dimension.height)
            ),
            format!(
                "scaleFactor: {}",
                java::format_fixed(header.scale_factor, 4)
            ),
            format!("seed: {}", header.seed),
            format!("svgLinkTarget: {}", optional(&header.svg_link_target)),
            format!(
                "hoverPathColorRGB: {}",
                optional(&header.hover_path_color_rgb)
            ),
            format!("preserveAspectRatio: {}", header.preserve_aspect_ratio),
            String::new(),
        ];
        header_lines
            .iter()
            .chain(&self.lines)
            .flat_map(|line| [line.as_str(), "\n"])
            .collect()
    }

    fn out_text(&mut self, text: &UText, at: UTranslate) {
        self.lines.extend([
            "TEXT:".to_owned(),
            format!("  text: {}", text.text),
            format!("  position: {}", point(at.dx, at.dy)),
            // Rotated text is not ported yet.
            "  orientation: 0".to_owned(),
            format!("  font: {}", text.font.to_string_debug()),
            format!(
                "  color: {}",
                color_to_string(Some(text.font.color()), &self.render_date)
            ),
            format!(
                "  extendedColor: {}",
                color_to_string(text.font.extended_color(), &self.render_date)
            ),
            String::new(),
        ]);
    }

    fn out_ellipse(&mut self, ellipse: &UEllipse, at: UTranslate, param: &UParam) {
        self.lines.extend([
            "ELLIPSE:".to_owned(),
            format!("  pt1: {}", point(at.dx, at.dy)),
            format!(
                "  pt2: {}",
                point(at.dx + ellipse.width, at.dy + ellipse.height)
            ),
            "  start: 0.0".to_owned(),
            "  extend: 0.0".to_owned(),
        ]);
        self.out_style(param);
    }

    fn out_rectangle(&mut self, rectangle: &URectangle, at: UTranslate, param: &UParam) {
        self.lines.extend([
            "RECTANGLE:".to_owned(),
            format!("  pt1: {}", point(at.dx, at.dy)),
            format!(
                "  pt2: {}",
                point(at.dx + rectangle.width, at.dy + rectangle.height)
            ),
            format!("  xCorner: {}", rectangle.rx as i32),
            format!("  yCorner: {}", rectangle.ry as i32),
        ]);
        self.out_style(param);
    }

    /// PlantUML lists a path's points without the current translation.
    fn out_path(&mut self, segments: &[USegment], param: &UParam) {
        self.lines.push("PATH:".to_owned());
        for segment in segments {
            match *segment {
                USegment::MoveTo(x, y) => self.out_segment("SEG_MOVETO", &[(x, y)]),
                USegment::LineTo(x, y) => self.out_segment("SEG_LINETO", &[(x, y)]),
                USegment::CubicTo { ctrl1, ctrl2, end } => {
                    self.out_segment("SEG_CUBICTO", &[ctrl1, ctrl2, end]);
                }
                USegment::ArcTo {
                    radius,
                    x_axis_rotation,
                    large_arc,
                    sweep,
                    end,
                } => self.lines.extend([
                    "   - type: SEG_ARCTO".to_owned(),
                    format!("     radius: {}", point(radius.0, radius.1)),
                    format!("     angle: {}", java::double_to_string(x_axis_rotation)),
                    format!("     largeArcFlag: {large_arc}"),
                    format!("     sweepFlag: {sweep}"),
                    format!("     dest: {}", point(end.0, end.1)),
                ]),
            }
        }
        self.out_style(param);
    }

    fn out_segment(&mut self, kind: &str, points: &[(f64, f64)]) {
        self.lines.push(format!("   - type: {kind}"));
        self.lines.extend(
            points
                .iter()
                .enumerate()
                .map(|(index, &(x, y))| format!("     pt{}: {}", index + 1, point(x, y))),
        );
    }

    /// Shadows are not ported yet, so every shape is listed without one.
    fn out_style(&mut self, param: &UParam) {
        self.lines.extend([
            format!("  stroke: {}", param.stroke),
            "  shadow: 0".to_owned(),
            format!(
                "  color: {}",
                color_to_string(Some(&param.color), &self.render_date)
            ),
            format!(
                "  backcolor: {}",
                color_to_string(Some(&param.backcolor), &self.render_date)
            ),
            String::new(),
        ]);
    }
}

/// Whether a clip drops a shape: rectangles, ellipses, lines and texts go when both their corners are out.
fn is_out_of_clip(shape: &UShape, at: UTranslate, clip: Option<&UClip>) -> bool {
    let Some(clip) = clip else {
        return false;
    };
    let (x, y) = (at.dx, at.dy);
    let both_out = |dx: f64, dy: f64| !clip.is_inside(x, y) && !clip.is_inside(x + dx, y + dy);
    match shape {
        UShape::Rectangle(rectangle) => both_out(rectangle.width, rectangle.height),
        UShape::Ellipse(ellipse) => both_out(ellipse.width, ellipse.height),
        UShape::Line { dx, dy } => both_out(*dx, *dy),
        UShape::Text(text) => {
            let dimension = StringBounderDebug.calculate_dimension(&text.font.font(), &text.text);
            both_out(dimension.width, -dimension.height)
        }
        UShape::Path(segments) => !clip.is_path_inside(x, y, segments),
        _ => false,
    }
}

impl UGraphicBackend for UGraphicDebug {
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
        if is_out_of_clip(shape, at, param.clip.as_ref()) {
            return;
        }
        match shape {
            UShape::Text(text) => self.out_text(text, at),
            UShape::Ellipse(ellipse) => self.out_ellipse(ellipse, at, param),
            UShape::Rectangle(rectangle) => self.out_rectangle(rectangle, at, param),
            UShape::Line { dx, dy } => {
                self.lines.extend([
                    "LINE:".to_owned(),
                    format!("  pt1: {}", point(at.dx, at.dy)),
                    format!("  pt2: {}", point(at.dx + dx, at.dy + dy)),
                    format!("  stroke: {}", param.stroke),
                    "  shadow: 0".to_owned(),
                    format!(
                        "  color: {}",
                        color_to_string(Some(&param.color), &self.render_date)
                    ),
                    String::new(),
                ]);
            }
            UShape::Polygon(points) => {
                self.lines
                    .extend(["POLYGON:".to_owned(), "  points:".to_owned()]);
                self.lines.extend(
                    points
                        .iter()
                        .map(|(x, y)| format!("   - {}", point(at.dx + x, at.dy + y))),
                );
                self.out_style(param);
            }
            UShape::Path(segments) => self.out_path(segments, param),
            UShape::Empty(dimension) => self.lines.extend([
                "EMPTY:".to_owned(),
                format!("  pt1: {}", point(at.dx, at.dy)),
                format!(
                    "  pt2: {}",
                    point(at.dx + dimension.width, at.dy + dimension.height)
                ),
                String::new(),
            ]),
            UShape::HorizontalLine | UShape::Image(_) => {
                let undescribed = format!(
                    "UGraphicDebug {} {}",
                    shape.java_class_name(),
                    self.render_date
                );
                self.lines.push(undescribed);
            }
        }
    }
}

fn point(x: f64, y: f64) -> String {
    format!(
        "[ {} ; {} ]",
        java::format_fixed(x, 4),
        java::format_fixed(y, 4)
    )
}

/// PlantUML describes colours other than plain ones by their class and the time.
fn color_to_string(color: Option<&HColor>, render_date: &str) -> String {
    match color {
        None => "NULL_COLOR".to_owned(),
        Some(color) if color.is_transparent() => "NULL_COLOR".to_owned(),
        Some(HColor::Simple(color)) => format!("{:x}", color.argb()),
        Some(other) => format!("{} {render_date}", other.java_class_name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Widths observed in PlantUML's debug output for Serif 14.
    #[test]
    fn text_is_measured_like_plantuml() {
        let bounder = StringBounderDebug;
        let font = UFont::serif(14);
        let hello = bounder.calculate_dimension(&font, "Hello world");
        assert_eq!(java::format_fixed(hello.height, 4), "14.0000");
        assert_eq!(bounder.descent(&font, "x"), 14.0 / 4.5);
        let tab = bounder.calculate_dimension(&font, "        ");
        assert_eq!(java::format_fixed(tab.width, 4), "133.5337");
    }
}
