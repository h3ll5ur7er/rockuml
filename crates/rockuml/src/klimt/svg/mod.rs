//! The SVG output format (PlantUML's `UGraphicSvg` and its shape drivers).

mod graphics;
mod xml;

use std::rc::Rc;

pub use graphics::SvgOption;
use graphics::{SvgGraphics, SvgText};

use super::font::{FontStyle, StringBounder};
use super::geom::UTranslate;
use super::group::UGroup;
use super::shape::{UImage, UShape, UText};
use super::ugraphic::{UGraphicBackend, UParam, UStroke};
use super::url::Url;
use crate::color::HColor;

pub struct UGraphicSvg {
    graphics: Option<SvgGraphics>,
    string_bounder: Rc<dyn StringBounder>,
}

impl UGraphicSvg {
    pub fn new(seed: i64, option: SvgOption, string_bounder: Rc<dyn StringBounder>) -> Self {
        Self {
            graphics: Some(SvgGraphics::new(seed, option)),
            string_bounder,
        }
    }

    /// The finished document, embedding `metadata` (the encoded source) when given.
    pub fn take_document(&mut self, metadata: Option<&str>) -> String {
        self.graphics
            .take()
            .expect("the document is taken once")
            .into_xml(metadata)
    }

    fn svg(&mut self) -> &mut SvgGraphics {
        self.graphics
            .as_mut()
            .expect("drawing before the document is taken")
    }

    fn apply_colors_and_stroke(&mut self, param: &UParam) {
        let fill = self.paint(&param.backcolor);
        let stroke = self.paint(&param.color);
        let svg = self.svg();
        svg.set_fill_color(Some(&fill));
        svg.set_stroke_color(Some(&stroke));
        apply_stroke(svg, param.stroke);
    }

    /// How SVG paints a colour: gradients by reference to their definition.
    fn paint(&mut self, color: &HColor) -> String {
        match color {
            HColor::Gradient(gradient) => self.svg().gradient_fill(*gradient),
            other => other.to_svg(),
        }
    }

    fn draw_text(&mut self, shape: &UText, at: UTranslate) {
        let configuration = &shape.font;
        if configuration.color().is_transparent() {
            return;
        }
        let font = configuration.font();
        let face = configuration.base_face();
        let font_weight = if configuration.contains_style(FontStyle::Bold) {
            Some(if face.weight >= 700 {
                face.weight.to_string()
            } else {
                "700".to_owned()
            })
        } else {
            (face.weight != 400).then(|| face.weight.to_string())
        };
        let font_style =
            (configuration.contains_style(FontStyle::Italic) || face.italic).then_some("italic");

        let mut x = at.dx;
        let mut text = shape.text.clone();
        if text.chars().all(crate::java::is_regex_whitespace) {
            text = text.replace(' ', "\u{A0}");
        }
        let leading_spaces = text.len() - text.trim_start_matches(' ').len();
        if leading_spaces > 0 {
            x += leading_spaces as f64 * self.string_bounder.calculate_dimension(&font, " ").width;
        }
        let text = crate::java::trim(&text).to_owned();
        let dimension = self.string_bounder.calculate_dimension(&font, &text);

        let mut decorations = Vec::new();
        let mut extra_lines = Vec::new();
        let size = font.size_2d();
        if configuration.contains_style(FontStyle::Underline)
            && configuration.underline_stroke().thickness > 0.0
        {
            match configuration.extended_color() {
                None => decorations.push("underline"),
                Some(color) => extra_lines.push((color.clone(), size / 14.0)),
            }
        }
        if configuration.contains_style(FontStyle::Strike) {
            match configuration.extended_color() {
                None => decorations.push("line-through"),
                Some(color) => extra_lines.push((color.clone(), -size / 4.0)),
            }
        }
        if configuration.contains_style(FontStyle::Wave) {
            decorations.push("wavy underline");
        }
        let mut back_color = None;
        if configuration.contains_style(FontStyle::Backcolor) {
            match configuration.extended_color() {
                Some(HColor::Gradient(gradient)) => {
                    // A filter floods with one colour, so gradients go on a rectangle a little below the line.
                    const PATCH: f64 = 2.0;
                    let fill = self.svg().gradient_fill(*gradient);
                    let svg = self.svg();
                    svg.set_fill_color(Some(&fill));
                    svg.set_stroke_color(None);
                    svg.rectangle(
                        x,
                        at.dy - dimension.height + PATCH,
                        dimension.width,
                        dimension.height,
                        0.0,
                        0.0,
                    );
                }
                Some(color) => back_color = Some(color.to_rgb()),
                None => {}
            }
        }

        let svg = self.svg();
        svg.set_fill_color(Some(&configuration.color().to_svg()));
        svg.text(&SvgText {
            text: &text,
            x,
            y: at.dy,
            font_family: &font.svg_family(),
            font_size: font.size(),
            font_weight,
            font_style,
            text_decoration: (!decorations.is_empty()).then(|| decorations.join(" ")),
            text_length: dimension.width,
            back_color,
        });
        for (color, delta_y) in extra_lines {
            svg.set_stroke_color(Some(&color.to_svg()));
            svg.set_stroke_width(size / 28.0, None);
            let y = at.dy + delta_y;
            svg.line(x, y, x + dimension.width, y);
        }
    }

    fn draw_image(&mut self, image: &UImage, at: UTranslate) {
        self.svg()
            .png_image(image.png, at.dx, at.dy, image.width, image.height);
    }
}

fn apply_stroke(svg: &mut SvgGraphics, stroke: UStroke) {
    let dasharray =
        (stroke.dash_visible != 0.0).then_some((stroke.dash_visible, stroke.dash_space));
    svg.set_stroke_width(stroke.thickness, dasharray);
}

impl UGraphicBackend for UGraphicSvg {
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
        match shape {
            UShape::Text(text) => self.draw_text(text, at),
            UShape::Rectangle(rectangle) => {
                self.apply_colors_and_stroke(param);
                self.svg().rectangle(
                    at.dx,
                    at.dy,
                    rectangle.width,
                    rectangle.height,
                    rectangle.rx / 2.0,
                    rectangle.ry / 2.0,
                );
            }
            UShape::Ellipse(ellipse) => {
                self.apply_colors_and_stroke(param);
                let (x_radius, y_radius) = (ellipse.width / 2.0, ellipse.height / 2.0);
                self.svg()
                    .ellipse(at.dx + x_radius, at.dy + y_radius, x_radius, y_radius);
            }
            UShape::Line { dx, dy } => {
                let svg = self.svg();
                svg.set_stroke_color(Some(&param.color.to_svg()));
                apply_stroke(svg, param.stroke);
                svg.line(at.dx, at.dy, at.dx + dx, at.dy + dy);
            }
            UShape::Polygon(points) => {
                self.apply_colors_and_stroke(param);
                let points: Vec<(f64, f64)> =
                    points.iter().map(|(x, y)| (at.dx + x, at.dy + y)).collect();
                self.svg().polygon(&points);
            }
            UShape::Path(segments) => {
                // A path filled with the colour of its outline gets no outline in PlantUML.
                if param.color == param.backcolor {
                    let svg = self.svg();
                    svg.set_fill_color(Some(&param.color.to_svg()));
                    svg.set_stroke_color(Some(""));
                    svg.set_stroke_width(0.0, None);
                } else {
                    self.apply_colors_and_stroke(param);
                }
                self.svg().path(at.dx, at.dy, segments);
            }
            UShape::Image(image) => self.draw_image(image, at),
            UShape::Empty(_) | UShape::HorizontalLine => {}
        }
    }

    fn start_group(&mut self, group: &UGroup) {
        self.svg().start_group(group);
    }

    fn close_group(&mut self) {
        self.svg().close_group();
    }

    fn start_url(&mut self, url: &Url) {
        self.svg().open_link(&url.href, &url.tooltip);
    }

    fn close_url(&mut self) {
        self.svg().close_link();
    }
}
