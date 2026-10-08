//! The SVG output format (PlantUML's `UGraphicSvg` and its shape drivers).

mod graphics;
mod xml;

use std::rc::Rc;
use std::sync::Arc;

pub(crate) use graphics::SvgOption;
use graphics::{SvgGraphics, SvgText};

use super::font::{FontStyle, StringBounder};
use super::geom::UTranslate;
use super::group::UGroup;
use super::shape::{UCenteredCharacter, UEllipse, UImage, UShape, UText};
use super::typeface::FontRegistry;
use super::ugraphic::{UGraphicBackend, UParam, UStroke};
use super::url::Url;
use crate::color::HColor;

pub(crate) struct UGraphicSvg {
    graphics: Option<SvgGraphics>,
    string_bounder: Rc<dyn StringBounder>,
    /// The fonts whose glyph outlines draw centred characters; deterministic SVG has none and writes them as
    /// text.
    glyph_fonts: Option<Arc<FontRegistry>>,
    /// The document becomes a PNG, whose `Graphics2D` driver in PlantUML draws no SVG images.
    rasterized: bool,
}

impl UGraphicSvg {
    pub(crate) fn new(
        seed: i64,
        option: SvgOption,
        string_bounder: Rc<dyn StringBounder>,
        glyph_fonts: Option<Arc<FontRegistry>>,
        rasterized: bool,
    ) -> Self {
        Self {
            graphics: Some(SvgGraphics::new(seed, option)),
            string_bounder,
            glyph_fonts,
            rasterized,
        }
    }

    /// The finished document, embedding `metadata` (the encoded source) when given.
    pub(crate) fn take_document(&mut self, metadata: Option<&str>) -> String {
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
        if param.backcolor == HColor::TransparentFill {
            svg.set_invisible_fill();
        } else {
            svg.set_fill_color(Some(&fill));
        }
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

    /// The glyph's outline, centred on its pixels. Deterministic SVG writes the character as text in a fixed
    /// font instead, since outlines depend on the font.
    fn draw_centered_character(
        &mut self,
        centered: &UCenteredCharacter,
        at: UTranslate,
        param: &UParam,
    ) {
        let color = param.color.to_svg();
        if let Some(fonts) = &self.glyph_fonts {
            if let Some(outline) = fonts.glyph_outline(&centered.font, centered.character) {
                let (center_x, center_y) = outline.center();
                let svg = self.svg();
                svg.set_fill_color(Some(&color));
                svg.glyph_path(
                    at.dx - center_x - 0.5,
                    at.dy - center_y - 0.5,
                    &outline.segments,
                );
            }
            return;
        }
        let svg = self.svg();
        svg.set_fill_color(Some(&color));
        svg.text(&SvgText {
            text: &centered.character.to_string(),
            x: at.dx - 5.0,
            y: at.dy + 5.0,
            font_family: "monospace",
            font_size: 14,
            font_weight: None,
            font_style: None,
            text_decoration: None,
            text_length: 0.0,
            back_color: None,
        });
    }

    fn draw_ellipse(&mut self, ellipse: &UEllipse, at: UTranslate) {
        let (x_radius, y_radius) = (ellipse.width / 2.0, ellipse.height / 2.0);
        let (cx, cy) = (at.dx + x_radius, at.dy + y_radius);
        if !ellipse.is_arc() {
            self.svg().ellipse(cx, cy, x_radius, y_radius);
            return;
        }
        // With sine for x and cosine for y, a quarter turn more keeps AWT's angles: counter-clockwise from
        // three o'clock.
        let start = ellipse.start + 90.0;
        let on_ellipse = |degrees: f64| {
            let radians = degrees * std::f64::consts::PI / 180.0;
            (
                cx + radians.sin() * ellipse.width / 2.0,
                cy + radians.cos() * ellipse.height / 2.0,
            )
        };
        let (from, to) = if ellipse.extend > 0.0 {
            (start, start + ellipse.extend)
        } else {
            (start + ellipse.extend, start)
        };
        self.svg()
            .arc_ellipse(x_radius, y_radius, on_ellipse(from), on_ellipse(to));
    }

    /// The pixels drawn, re-encoded as PNG as PlantUML does.
    fn draw_image(&mut self, image: &UImage, at: UTranslate) {
        let Some(pixels) = image.image() else {
            return;
        };
        self.svg().png_image(
            &pixels.to_png(),
            at.dx,
            at.dy,
            pixels.width() as f64,
            pixels.height() as f64,
        );
    }
}

fn apply_stroke(svg: &mut SvgGraphics, stroke: UStroke) {
    let dasharray =
        (stroke.dash_visible != 0.0).then_some((stroke.dash_visible, stroke.dash_space));
    svg.set_stroke_width(stroke.thickness, dasharray);
}

impl UGraphicBackend for UGraphicSvg {
    fn draws_special_text(&self) -> bool {
        true
    }

    /// Clips as PlantUML's SVG drivers do: straight lines and rectangles are cut to the clip, other shapes
    /// are dropped unless inside.
    fn draw(&mut self, shape: &UShape, at: UTranslate, param: &UParam) {
        let clip = param.clip.as_ref();
        let inside = |x: f64, y: f64| clip.is_none_or(|clip| clip.is_inside(x, y));
        match shape {
            UShape::Text(text) => {
                if inside(at.dx, at.dy) {
                    self.draw_text(text, at);
                }
            }
            UShape::Rectangle(rectangle) => {
                let (x, y, width, height) = match clip {
                    Some(clip) => {
                        clip.clipped_rectangle(at.dx, at.dy, rectangle.width, rectangle.height)
                    }
                    None => (at.dx, at.dy, rectangle.width, rectangle.height),
                };
                if clip.is_some() && height <= 0.0 {
                    return;
                }
                self.apply_colors_and_stroke(param);
                self.svg()
                    .rectangle(x, y, width, height, rectangle.rx / 2.0, rectangle.ry / 2.0);
            }
            UShape::Ellipse(ellipse) => {
                if !inside(at.dx, at.dy) || !inside(at.dx + ellipse.width, at.dy + ellipse.height) {
                    return;
                }
                self.apply_colors_and_stroke(param);
                self.draw_ellipse(ellipse, at);
            }
            UShape::Line { dx, dy } => {
                let start = (at.dx, at.dy);
                let end = (at.dx + dx, at.dy + dy);
                let Some(((x1, y1), (x2, y2))) = (match clip {
                    Some(clip) => clip.clipped_line(start, end),
                    None => Some((start, end)),
                }) else {
                    return;
                };
                let svg = self.svg();
                svg.set_stroke_color(Some(&param.color.to_svg()));
                apply_stroke(svg, param.stroke);
                svg.line(x1, y1, x2, y2);
            }
            UShape::Polygon(polygon) => {
                let points: Vec<(f64, f64)> = polygon
                    .points()
                    .iter()
                    .map(|(x, y)| (at.dx + x, at.dy + y))
                    .collect();
                if !points.iter().all(|&(x, y)| inside(x, y)) {
                    return;
                }
                self.apply_colors_and_stroke(param);
                self.svg().polygon(&points);
            }
            UShape::Path(segments) => {
                if clip.is_some_and(|clip| !clip.is_path_inside(at.dx, at.dy, segments)) {
                    return;
                }
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
            UShape::Image(image) => {
                if inside(at.dx, at.dy) && inside(at.dx + image.width(), at.dy + image.height()) {
                    self.draw_image(image, at);
                }
            }
            UShape::ImageSvg(image) => {
                if !self.rasterized {
                    self.svg().svg_image(image, at.dx, at.dy);
                }
            }
            UShape::CenteredCharacter(centered) => {
                self.draw_centered_character(centered, at, param);
            }
            UShape::Comment(comment) => self.svg().add_comment(comment),
            UShape::Empty(_)
            | UShape::HorizontalLine
            | UShape::SpecialText
            | UShape::CenteredText(_) => {}
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
