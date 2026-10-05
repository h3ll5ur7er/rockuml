//! Builds the SVG document shape by shape (PlantUML's `SvgGraphics`).

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

use regex::Regex;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;

use super::xml::XmlNode;
use crate::java;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::shape::USegment;

const DEFAULT_FONT_FAMILY: &str = "sans-serif";
const DECIMALS: usize = 3;

/// The document-wide choices made before drawing starts.
pub struct SvgOption {
    /// The whole image's size; the document grows beyond it when shapes stick out.
    pub min_dim: XDimension2D,
    /// `None` for a transparent image.
    pub backcolor: Option<String>,
    /// Every length is multiplied by it on output.
    pub scale: f64,
    pub preserve_aspect_ratio: String,
    /// Extra attributes of the `<svg>` element, like `data-diagram-type`.
    pub root_attributes: Vec<(String, String)>,
    /// The window links open in.
    pub link_target: Option<String>,
}

/// A text run with everything that styles it.
pub struct SvgText<'a> {
    pub text: &'a str,
    pub x: f64,
    pub y: f64,
    pub font_family: &'a str,
    pub font_size: i32,
    pub font_weight: Option<String>,
    pub font_style: Option<&'static str>,
    pub text_decoration: Option<String>,
    pub text_length: f64,
    pub back_color: Option<String>,
}

pub struct SvgGraphics {
    option: SvgOption,
    defs: XmlNode,
    g_root: XmlNode,
    /// Groups and links not closed yet, innermost last; shapes go into the innermost.
    open_elements: Vec<XmlNode>,
    /// Links not closed yet, innermost last. SVG links cannot nest, so only the innermost is open, and it is
    /// reopened around groups started inside it.
    active_links: Vec<Link>,
    fill: String,
    stroke: String,
    stroke_width: String,
    stroke_dasharray: Option<String>,
    max_x: i32,
    max_y: i32,
    filter_uid: String,
    /// Text background colours and the filters that paint them.
    back_color_filters: HashMap<String, String>,
    painted_background: bool,
}

impl SvgGraphics {
    pub fn new(seed: i64, option: SvgOption) -> Self {
        let mut g_root = XmlNode::new("g");
        g_root.set_attribute("font-family", DEFAULT_FONT_FAMILY);
        g_root.set_attribute("lengthAdjust", "spacing");
        let mut graphics = Self {
            defs: XmlNode::new("defs"),
            g_root,
            open_elements: Vec::new(),
            active_links: Vec::new(),
            fill: "black".to_owned(),
            stroke: "black".to_owned(),
            stroke_width: String::new(),
            stroke_dasharray: None,
            max_x: 10,
            max_y: 10,
            filter_uid: format!("b{}", radix36(seed.unsigned_abs())),
            back_color_filters: HashMap::new(),
            painted_background: false,
            option,
        };
        graphics.stroke_width = graphics.length(1.0);
        let XDimension2D { width, height } = graphics.option.min_dim;
        graphics.ensure_visible(width, height);
        if let Some(color) = graphics.option.backcolor.clone()
            && !["#00000000", "#000000", "#FFFFFF"].contains(&color.as_str())
        {
            graphics.paint_background(&color);
        }
        graphics
    }

    /// A rectangle behind everything, sized to the whole document once it is known.
    fn paint_background(&mut self, color: &str) {
        self.set_fill_color(Some(color));
        self.set_stroke_color(None);
        let background = self.rectangle_element(0.0, 0.0, 0.0, 0.0);
        self.g_root.append_child(background);
        self.painted_background = true;
    }

    /// A length on output: scaled, three decimals without trailing zeros.
    fn length(&self, value: f64) -> String {
        decimal(value * self.option.scale)
    }

    fn ensure_visible(&mut self, x: f64, y: f64) {
        if x > f64::from(self.max_x) {
            self.max_x = (x + 1.0) as i32;
        }
        if y > f64::from(self.max_y) {
            self.max_y = (y + 1.0) as i32;
        }
    }

    /// `None` and fully transparent colours paint nothing.
    pub fn set_fill_color(&mut self, color: Option<&str>) {
        self.fill = fix_color(color);
    }

    pub fn set_stroke_color(&mut self, color: Option<&str>) {
        self.stroke = fix_color(color);
    }

    pub fn set_stroke_width(&mut self, width: f64, dasharray: Option<(f64, f64)>) {
        self.stroke_width = self.length(width);
        self.stroke_dasharray = dasharray
            .map(|(visible, space)| format!("{},{}", self.length(visible), self.length(space)));
    }

    fn current_group(&mut self) -> &mut XmlNode {
        self.open_elements.last_mut().unwrap_or(&mut self.g_root)
    }

    fn fill_me(&self, element: &mut XmlNode) {
        let is_argb = self.fill.len() == 9
            && self.fill.starts_with('#')
            && self.fill[1..].bytes().all(|byte| byte.is_ascii_hexdigit());
        if is_argb {
            element.set_attribute("fill", shorten_color(&self.fill[..7]));
            let alpha = u8::from_str_radix(&self.fill[7..], 16).expect("checked hex digits");
            element.set_attribute("fill-opacity", opacity(f64::from(alpha) / 255.0));
        } else {
            element.set_attribute("fill", shorten_color(&self.fill));
        }
    }

    fn style_me(&self, element: &mut XmlNode, extra_style: &str) {
        if self.stroke_width == "0" {
            return;
        }
        let mut style = format!("stroke:{};", shorten_color(&self.stroke));
        if self.stroke != "none" {
            write!(style, "stroke-width:{};", self.stroke_width).expect("writing to a String");
            if let Some(dasharray) = &self.stroke_dasharray {
                write!(style, "stroke-dasharray:{dasharray};").expect("writing to a String");
            }
        }
        style.push_str(extra_style);
        element.set_attribute("style", style);
    }

    fn rectangle_element(&self, x: f64, y: f64, width: f64, height: f64) -> XmlNode {
        let mut element = XmlNode::new("rect");
        element.set_attribute("x", self.length(x));
        element.set_attribute("y", self.length(y));
        element.set_attribute("width", self.length(width));
        element.set_attribute("height", self.length(height));
        self.fill_me(&mut element);
        self.style_me(&mut element, "");
        element
    }

    pub fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64) {
        if height <= 0.0 || width <= 0.0 {
            return;
        }
        let mut element = self.rectangle_element(x, y, width, height);
        if rx > 0.0 && ry > 0.0 {
            element.set_attribute("rx", self.length(rx));
            element.set_attribute("ry", self.length(ry));
        }
        self.current_group().append_child(element);
        self.ensure_visible(x + width, y + height);
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let mut element = XmlNode::new("line");
        element.set_attribute("x1", self.length(x1));
        element.set_attribute("y1", self.length(y1));
        element.set_attribute("x2", self.length(x2));
        element.set_attribute("y2", self.length(y2));
        self.style_me(&mut element, "");
        self.current_group().append_child(element);
        self.ensure_visible(x1, y1);
        self.ensure_visible(x2, y2);
    }

    pub fn ellipse(&mut self, x: f64, y: f64, x_radius: f64, y_radius: f64) {
        let mut element = XmlNode::new("ellipse");
        element.set_attribute("cx", self.length(x));
        element.set_attribute("cy", self.length(y));
        element.set_attribute("rx", self.length(x_radius));
        element.set_attribute("ry", self.length(y_radius));
        self.fill_me(&mut element);
        self.style_me(&mut element, "");
        self.current_group().append_child(element);
        self.ensure_visible(x + x_radius, y + y_radius);
    }

    pub fn polygon(&mut self, points: &[(f64, f64)]) {
        let mut element = XmlNode::new("polygon");
        let coordinates: Vec<String> = points
            .iter()
            .flat_map(|&(x, y)| [self.length(x), self.length(y)])
            .collect();
        element.set_attribute("points", coordinates.join(","));
        self.fill_me(&mut element);
        self.style_me(&mut element, "stroke-linejoin:miter;stroke-miterlimit:10;");
        self.current_group().append_child(element);
        for &(x, y) in points {
            self.ensure_visible(x, y);
        }
    }

    pub fn path(&mut self, x: f64, y: f64, segments: &[USegment]) {
        self.ensure_visible(x, y);
        let mut d = Vec::with_capacity(segments.len());
        for segment in segments {
            d.push(match *segment {
                USegment::MoveTo(dx, dy) => format!("M{}", self.visible_point(x + dx, y + dy)),
                USegment::LineTo(dx, dy) => format!("L{}", self.visible_point(x + dx, y + dy)),
                USegment::CubicTo { ctrl1, ctrl2, end } => format!(
                    "C{} {} {}",
                    self.visible_point(x + ctrl1.0, y + ctrl1.1),
                    self.visible_point(x + ctrl2.0, y + ctrl2.1),
                    self.visible_point(x + end.0, y + end.1)
                ),
                USegment::ArcTo {
                    radius,
                    x_axis_rotation,
                    large_arc,
                    sweep,
                    end,
                } => {
                    self.ensure_visible(end.0 + radius.0 + x, end.1 + radius.1 + y);
                    format!(
                        "A{},{} {} {} {} {},{}",
                        self.length(radius.0),
                        self.length(radius.1),
                        self.length(x_axis_rotation),
                        u8::from(large_arc),
                        u8::from(sweep),
                        self.length(end.0 + x),
                        self.length(end.1 + y)
                    )
                }
            });
        }
        let mut element = XmlNode::new("path");
        element.set_attribute("d", d.join(" "));
        self.style_me(&mut element, "");
        self.fill_me(&mut element);
        self.current_group().append_child(element);
    }

    /// `x,y`, after growing the image to show the point.
    fn visible_point(&mut self, x: f64, y: f64) -> String {
        self.ensure_visible(x, y);
        format!("{},{}", self.length(x), self.length(y))
    }

    pub fn text(&mut self, text: &SvgText) {
        let mut element = XmlNode::new("text");
        element.set_attribute("x", self.length(text.x));
        element.set_attribute("y", self.length(text.y));
        self.fill_me(&mut element);
        element.set_attribute("font-size", self.length(f64::from(text.font_size)));
        if text.text.encode_utf16().nth(1).is_some() {
            element.set_attribute("textLength", self.length(text.text_length));
        }
        if let Some(weight) = &text.font_weight {
            element.set_attribute("font-weight", weight.as_str());
        }
        if let Some(style) = text.font_style {
            element.set_attribute("font-style", style);
        }
        if let Some(decoration) = &text.text_decoration {
            element.set_attribute("text-decoration", decoration.as_str());
        }
        let family = if text.font_family.eq_ignore_ascii_case("monospaced") {
            "monospace"
        } else {
            text.font_family
        };
        if !family.eq_ignore_ascii_case(DEFAULT_FONT_FAMILY) {
            element.set_attribute("font-family", family);
        }
        let content =
            if family.eq_ignore_ascii_case("monospace") || family.eq_ignore_ascii_case("courier") {
                text.text.replace(' ', "\u{A0}")
            } else {
                text.text.to_owned()
            };
        if let Some(color) = &text.back_color {
            let filter = self.back_color_filter(color);
            element.set_attribute("filter", format!("url(#{filter})"));
        }
        element.set_text_content(&content);
        self.current_group().append_child(element);
        self.ensure_visible(text.x, text.y);
        self.ensure_visible(text.x + text.text_length, text.y);
    }

    /// The id of a filter flooding the text's box with `color`, created on first use.
    fn back_color_filter(&mut self, color: &str) -> String {
        if let Some(id) = self.back_color_filters.get(color) {
            return id.clone();
        }
        let id = format!("{}{}", self.filter_uid, self.back_color_filters.len());
        self.back_color_filters.insert(color.to_owned(), id.clone());
        let mut filter = XmlNode::new("filter");
        for (name, value) in [
            ("id", id.as_str()),
            ("x", "0"),
            ("y", "0"),
            ("width", "1"),
            ("height", "1"),
        ] {
            filter.set_attribute(name, value);
        }
        let mut flood = XmlNode::new("feFlood");
        flood.set_attribute("flood-color", color);
        flood.set_attribute("result", "flood");
        filter.append_child(flood);
        let mut composite = XmlNode::new("feComposite");
        composite.set_attribute("in", "SourceGraphic");
        composite.set_attribute("in2", "flood");
        composite.set_attribute("operator", "over");
        filter.append_child(composite);
        self.defs.append_child(filter);
        id
    }

    /// An image embedded as a PNG data URI.
    pub fn png_image(&mut self, png: &[u8], x: f64, y: f64, width: f64, height: f64) {
        let mut element = XmlNode::new("image");
        element.set_attribute("width", self.length(width));
        element.set_attribute("height", self.length(height));
        element.set_attribute("x", self.length(x));
        element.set_attribute("y", self.length(y));
        element.set_attribute(
            "xlink:href",
            format!("data:image/png;base64,{}", BASE64_STANDARD.encode(png)),
        );
        self.current_group().append_child(element);
        self.ensure_visible(x, y);
        self.ensure_visible(x + width, y + height);
    }

    pub fn start_group(&mut self, group: &UGroup) {
        self.close_innermost_link_element();
        let mut element = XmlNode::new("g");
        for (kind, value) in group.entries() {
            element.set_attribute(kind.svg_attribute_name(), value);
        }
        self.open_elements.push(element);
        self.reopen_innermost_link();
    }

    pub fn close_group(&mut self) {
        self.close_innermost_link_element();
        self.close_innermost_element();
        self.reopen_innermost_link();
    }

    pub fn open_link(&mut self, url: &str, tooltip: &str) {
        self.close_innermost_link_element();
        self.active_links.push(Link {
            url: if is_javascript(url) {
                String::new()
            } else {
                url.to_owned()
            },
            title: decoded_title(tooltip),
            target: self.option.link_target.clone().unwrap_or_default(),
        });
        self.reopen_innermost_link();
    }

    pub fn close_link(&mut self) {
        self.close_innermost_link_element();
        self.active_links.pop().expect("a link is open");
        self.reopen_innermost_link();
    }

    /// Empty elements are dropped.
    fn close_innermost_element(&mut self) {
        let element = self.open_elements.pop().expect("an element is open");
        if element.has_children() {
            self.current_group().append_child(element);
        }
    }

    /// While a link is active, the innermost open element is its `<a>`.
    fn close_innermost_link_element(&mut self) {
        if !self.active_links.is_empty() {
            self.close_innermost_element();
        }
    }

    fn reopen_innermost_link(&mut self) {
        if let Some(link) = self.active_links.last() {
            self.open_elements.push(link.element());
        }
    }

    /// The finished document; `metadata` is the encoded diagram source PlantUML embeds.
    pub fn into_xml(mut self, metadata: Option<&str>) -> String {
        assert!(
            self.open_elements.is_empty(),
            "every group and link is closed"
        );
        if let Some(metadata) = metadata {
            self.g_root
                .append_processing_instruction("plantuml-src", metadata);
        }
        if self.painted_background {
            let (width, height) = (
                self.length(f64::from(self.max_x)),
                self.length(f64::from(self.max_y)),
            );
            let background = self
                .g_root
                .first_element_mut()
                .expect("the background is painted first");
            background.set_attribute("width", width);
            background.set_attribute("height", height);
        }
        let mut svg = XmlNode::new("svg");
        svg.set_attribute("xmlns", "http://www.w3.org/2000/svg");
        svg.set_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        svg.set_attribute("version", "1.1");
        for (name, value) in &self.option.root_attributes {
            svg.set_attribute(name, value.as_str());
        }
        let scaled = |max: i32| (f64::from(max) * self.option.scale) as i32;
        let (width, height) = (scaled(self.max_x), scaled(self.max_y));
        let mut style = format!("width:{width}px;height:{height}px;");
        if let Some(color) = self
            .option
            .backcolor
            .as_deref()
            .filter(|color| *color != "#00000000")
        {
            write!(style, "background:{color};").expect("writing to a String");
        }
        svg.set_attribute("style", style);
        svg.set_attribute("width", format!("{}px", self.length(f64::from(self.max_x))));
        svg.set_attribute(
            "height",
            format!("{}px", self.length(f64::from(self.max_y))),
        );
        svg.set_attribute("viewBox", format!("0 0 {width} {height}"));
        svg.set_attribute("zoomAndPan", "magnify");
        svg.set_attribute(
            "preserveAspectRatio",
            self.option.preserve_aspect_ratio.as_str(),
        );
        svg.set_attribute("contentStyleType", "text/css");
        svg.append_processing_instruction("plantuml", crate::PLANTUML_VERSION);
        svg.append_child(self.defs);
        svg.append_child(self.g_root);
        svg.to_xml()
    }
}

/// Three decimals without trailing zeros.
fn decimal(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    trim_zeros(&java::format_fixed(value, DECIMALS))
}

fn opacity(value: f64) -> String {
    if value <= 0.0 {
        "0".to_owned()
    } else if value >= 1.0 {
        "1".to_owned()
    } else {
        trim_zeros(&java::format_fixed(value, DECIMALS))
    }
}

struct Link {
    url: String,
    title: String,
    target: String,
}

impl Link {
    fn element(&self) -> XmlNode {
        let mut element = XmlNode::new("a");
        element.set_attribute("target", self.target.as_str());
        element.set_attribute("href", self.url.as_str());
        element.set_attribute("xlink:href", self.url.as_str());
        element.set_attribute("xlink:type", "simple");
        element.set_attribute("xlink:actuate", "onRequest");
        element.set_attribute("xlink:show", "new");
        element.set_attribute("title", self.title.as_str());
        element.set_attribute("xlink:title", self.title.as_str());
        element
    }
}

/// PlantUML drops `javascript:` links, however they are disguised.
fn is_javascript(url: &str) -> bool {
    url.to_lowercase()
        .chars()
        .filter(char::is_ascii_lowercase)
        .collect::<String>()
        .starts_with("javascript")
}

/// `<U+XXXX>` becomes the character, and a written `\n` a line break.
fn decoded_title(tooltip: &str) -> String {
    static UNICODE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<U\+([0-9A-Fa-f]+)>").expect("valid"));
    let decoded = UNICODE.replace_all(tooltip, |captures: &regex::Captures<'_>| {
        u32::from_str_radix(&captures[1], 16)
            .ok()
            .and_then(|code| char::from_u32(code & 0xFFFF))
            .map(String::from)
            .unwrap_or_default()
    });
    decoded.replace("\\n", "\n")
}

fn fix_color(color: Option<&str>) -> String {
    match color {
        None | Some("#00000000") => "none".to_owned(),
        Some(color) => color.to_owned(),
    }
}

/// `#RRGGBB` as `#RGB` when each channel's two digits are the same.
fn shorten_color(color: &str) -> String {
    let bytes = color.as_bytes();
    if bytes.len() == 7
        && bytes[0] == b'#'
        && bytes[1] == bytes[2]
        && bytes[3] == bytes[4]
        && bytes[5] == bytes[6]
    {
        format!(
            "#{}{}{}",
            bytes[1] as char, bytes[3] as char, bytes[5] as char
        )
    } else {
        color.to_owned()
    }
}

/// Java's `Long.toString(value, 36)`.
fn radix36(mut value: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut digits = Vec::new();
    loop {
        digits.push(DIGITS[(value % 36) as usize]);
        value /= 36;
        if value == 0 {
            break;
        }
    }
    digits.reverse();
    String::from_utf8(digits).expect("ASCII digits")
}

fn trim_zeros(number: &str) -> String {
    match number.split_once('.') {
        Some((integer, fraction)) => {
            let fraction = fraction.trim_end_matches('0');
            if fraction.is_empty() {
                integer.to_owned()
            } else {
                format!("{integer}.{fraction}")
            }
        }
        None => number.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graphics() -> SvgGraphics {
        SvgGraphics::new(
            42,
            SvgOption {
                min_dim: XDimension2D::new(20.0, 30.0),
                backcolor: Some("#FFFFFF".to_owned()),
                scale: 1.0,
                preserve_aspect_ratio: "none".to_owned(),
                root_attributes: Vec::new(),
                link_target: None,
            },
        )
    }

    #[test]
    fn numbers_have_three_decimals_without_trailing_zeros() {
        assert_eq!(decimal(0.0), "0");
        assert_eq!(decimal(14.6666666), "14.667");
        assert_eq!(decimal(2.5), "2.5");
        assert_eq!(decimal(3.0), "3");
    }

    #[test]
    fn ids_are_written_in_base_36() {
        assert_eq!(radix36(0), "0");
        assert_eq!(radix36(42), "16");
        assert_eq!(radix36(36 * 36), "100");
    }

    #[test]
    fn colours_are_shortened() {
        assert_eq!(shorten_color("#FFAA00"), "#FA0");
        assert_eq!(shorten_color("#FFAA01"), "#FFAA01");
        assert_eq!(shorten_color("none"), "none");
    }

    #[test]
    fn the_document_covers_the_minimum_size_and_every_shape() {
        let mut graphics = graphics();
        graphics.set_fill_color(Some("#FF0000"));
        graphics.set_stroke_color(Some("#000000"));
        graphics.rectangle(1.0, 2.0, 40.5, 3.0, 0.0, 0.0);
        let xml = graphics.into_xml(None);
        let size =
            r#"style="width:42px;height:31px;background:#FFFFFF;" width="42px" height="31px""#;
        assert!(xml.contains(size), "{xml}");
        let rectangle = r##"<rect x="1" y="2" width="40.5" height="3" fill="#F00" style="stroke:#000;stroke-width:1;"/>"##;
        assert!(xml.contains(rectangle), "{xml}");
    }

    #[test]
    fn links_cannot_nest_and_reopen_around_groups() {
        let mut graphics = graphics();
        graphics.open_link("https://a", "A");
        graphics.rectangle(0.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        graphics.start_group(&UGroup::default());
        graphics.rectangle(0.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        graphics.close_group();
        graphics.close_link();
        let xml = graphics.into_xml(None);
        assert_eq!(xml.matches("<a ").count(), 2, "{xml}");
        assert!(!xml.contains("<a target=\"\" href=\"https://a\" xlink:href=\"https://a\" xlink:type=\"simple\" xlink:actuate=\"onRequest\" xlink:show=\"new\" title=\"A\" xlink:title=\"A\"><g"), "{xml}");
    }

    #[test]
    fn link_titles_decode_references_and_javascript_is_dropped() {
        assert_eq!(decoded_title("a<U+221E>b\\nc"), "a\u{221E}b\nc");
        assert!(is_javascript("Java Script:alert(1)"));
        assert!(!is_javascript("https://plantuml.com"));
    }

    #[test]
    fn empty_groups_are_dropped() {
        let mut graphics = graphics();
        graphics.start_group(&UGroup::default());
        graphics.close_group();
        assert!(
            graphics
                .into_xml(None)
                .contains("<g font-family=\"sans-serif\" lengthAdjust=\"spacing\"/>")
        );
    }
}
