//! The members of a class or object, one per line, with their visibility icons in a column before them
//! (PlantUML's `MethodsOrFieldsArea`).

use super::Member;
use crate::color::Colors;
use crate::creole::{CreoleMode, Display};
use crate::klimt::blocks::{TextBlockLineBefore, TextBlockMarged};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D, XRectangle2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, Style, StyleBuilder, ValueReading};

/// A line of a body: a member, or text written in a block of an element that has no members.
#[derive(Clone)]
pub(crate) enum BodyLine {
    Member(Member),
    Text(String),
}

impl BodyLine {
    /// Separators like `--` and `== title ==` split bodies into blocks.
    pub(crate) fn as_str(&self) -> String {
        match self {
            Self::Member(member) => member.raw().to_owned(),
            Self::Text(text) => text.clone(),
        }
    }
}

/// A line drawn: its visibility icon, which takes no room without one, and its text.
struct Row {
    icon: Box<dyn TextBlock>,
    text: Box<dyn TextBlock>,
    /// The member as written, which notes on members name it by.
    raw: Option<String>,
}

pub(crate) struct MethodsOrFieldsArea {
    rows: Vec<Row>,
    /// The column of the text when there are icons; none otherwise.
    small_icon: Option<f64>,
    align: HorizontalAlignment,
}

impl MethodsOrFieldsArea {
    /// `skin` styles the icons with its rules in force at the end of the diagram; `style` and `colors`, the
    /// element's, give the text its font.
    pub(crate) fn new(
        members: &[BodyLine],
        skin: &SkinParam,
        style_builder: &StyleBuilder,
        style: &Style,
        colors: &Colors,
        align: HorizontalAlignment,
    ) -> Self {
        let icon_size = skin.class_attribute_icon_size();
        let has_small_icon = icon_size != 0
            && members.iter().any(|line| {
                matches!(line, BodyLine::Member(member) if member.get_visibility_modifier().is_some())
            });
        let rows = members
            .iter()
            .map(|line| Row {
                icon: match line {
                    BodyLine::Member(member) if has_small_icon => {
                        u_block(member, icon_size, style_builder)
                    }
                    _ => Box::new(NoIcon),
                },
                text: create_text_block(line, skin, style, colors, align),
                raw: match line {
                    BodyLine::Member(member) => Some(member.raw().to_owned()),
                    BodyLine::Text(_) => None,
                },
            })
            .collect();
        Self {
            rows,
            small_icon: has_small_icon.then(|| f64::from(skin.get_circled_character_radius() + 3)),
            align,
        }
    }

    /// The area under a separator line, with room around it (`asBlockMemberImpl`).
    pub(crate) fn as_block_member_impl(self, line_thickness: f64) -> Box<dyn TextBlock> {
        Box::new(TextBlockLineBefore {
            block: Box::new(TextBlockMarged::new(
                self,
                ClockwiseTopRightBottomLeft::top_right_bottom_left(4.0, 6.0, 4.0, 6.0),
            )),
            style: '\0',
            title: None,
            thickness: line_thickness,
        })
    }
}

impl TextBlock for MethodsOrFieldsArea {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let mut x: f64 = 0.0;
        let mut y = 0.0;
        for row in &self.rows {
            let dimension = row.text.calculate_dimension(string_bounder);
            x = x.max(dimension.width);
            y += dimension.height;
        }
        XDimension2D::new(x + self.small_icon.unwrap_or(0.0), y)
    }

    fn draw_u(&self, ug: &UGraphic) {
        for (row, (icon, text)) in self.rows.iter().zip(self.layout(ug.string_bounder())) {
            if let Some(icon) = icon {
                row.icon.draw_u(&ug.translated(icon.x, icon.y));
            }
            row.text.draw_u(&ug.translated(text.x, text.y));
        }
    }

    /// The member's text, with its icon when there are icons.
    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        let small_icon = self.small_icon.unwrap_or(0.0);
        self.rows
            .iter()
            .zip(self.layout(string_bounder))
            .find(|(row, _)| row.raw.as_deref() == Some(member))
            .map(|(row, (_, text))| {
                let dimension = row.text.calculate_dimension(string_bounder);
                XRectangle2D {
                    x: text.x - small_icon,
                    y: text.y,
                    width: dimension.width + small_icon,
                    height: dimension.height,
                }
            })
    }
}

impl MethodsOrFieldsArea {
    /// Where each row's icon, if drawn, and text go: icons in a column before the texts
    /// (`PlacementStrategyVisibility`), or else texts filling the height (`PlacementStrategyY1Y2`).
    fn layout(&self, string_bounder: &dyn StringBounder) -> Vec<(Option<XPoint2D>, XPoint2D)> {
        let width = self.calculate_dimension(string_bounder).width;
        let mut y = 0.0;
        let mut result = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let text = row.text.calculate_dimension(string_bounder);
            match self.small_icon {
                Some(column) => {
                    let icon = row.icon.calculate_dimension(string_bounder);
                    let height = icon.height.max(text.height);
                    result.push((
                        Some(XPoint2D::new(0.0, 2.0 + y + (height - icon.height) / 2.0)),
                        XPoint2D::new(column, y + (height - text.height) / 2.0),
                    ));
                    y += height;
                }
                None => {
                    result.push((None, XPoint2D::new(self.align.offset(width, text.width), y)));
                    y += text.height;
                }
            }
        }
        result
    }
}

/// The text of a line: a member in italics when abstract, underlined when static, with its visibility as a
/// character when there are no icons.
fn create_text_block(
    line: &BodyLine,
    skin: &SkinParam,
    style: &Style,
    colors: &Colors,
    align: HorizontalAlignment,
) -> Box<dyn TextBlock> {
    let mut config = style.font_configuration_with(colors);
    let text = match line {
        BodyLine::Member(member) => {
            let with_visibility_char = skin.class_attribute_icon_size() == 0;
            let mut text = member.get_display(with_visibility_char);
            if with_visibility_char && text.starts_with('#') {
                // A leading `#` would read as a colour.
                text.insert(0, '~');
            }
            if member.is_abstract() {
                config = config.with_style(crate::klimt::font::FontStyle::Italic);
            }
            if member.is_static() {
                config = config.with_style(crate::klimt::font::FontStyle::Underline);
            }
            text
        }
        BodyLine::Text(text) => text.clone(),
    };
    let block = Display::with_newlines(&text).create0(
        &config,
        align,
        skin,
        style.wrap_width(),
        CreoleMode::SimpleLine,
    );
    match line {
        BodyLine::Member(member) => with_url(Box::new(block), member.get_url()),
        BodyLine::Text(_) => Box::new(block),
    }
}

fn with_url(block: Box<dyn TextBlock>, url: Option<&Url>) -> Box<dyn TextBlock> {
    match url {
        Some(url) => Box::new(TextBlockWithUrl {
            block,
            url: url.clone(),
        }),
        None => block,
    }
}

/// The icon of a member's visibility, styled by the rules for visibility icons (`getUBlock`).
fn u_block(member: &Member, icon_size: i32, style_builder: &StyleBuilder) -> Box<dyn TextBlock> {
    let Some(modifier) = member.get_visibility_modifier() else {
        return Box::new(NoIcon);
    };
    let style = modifier
        .get_style_signature()
        .get_merged_style(style_builder);
    let border = style.value(PName::LineColor).as_color();
    let back = (!modifier.is_field()).then(|| style.value(PName::BackGroundColor).as_color());
    let url = member.get_url();
    with_url(
        Box::new(modifier.get_u_block(icon_size, border, back, url.is_some())),
        url,
    )
}

/// What stands in for the icon of a member without visibility.
struct NoIcon;

impl TextBlock for NoIcon {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(1.0, 1.0)
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}

/// A member's text that follows its link when clicked (`TextBlockTracer`).
struct TextBlockWithUrl {
    block: Box<dyn TextBlock>,
    url: Url,
}

impl TextBlock for TextBlockWithUrl {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.start_url(&self.url);
        self.block.draw_u(ug);
        ug.close_url();
    }
}
