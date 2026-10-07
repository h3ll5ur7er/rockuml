//! A map: a box with its name above a table of keys and values; a key linked to an entity spans the row
//! (PlantUML's `EntityImageMap` and `TextBlockMap`). JSON elements share the box.

use std::rc::Rc;

use super::entity_group;
use super::object::{draw_title, name_and_stereotype_dimension};
use crate::abel::{Entity, EntityPortion};
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, Display};
use crate::cucadiagram::Bodier;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::group::UGroup;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::font_param::FontParam;
use crate::skin::visibility_modifier::VisibilityModifier;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage};

/// The room each side of the title.
const X_MARGIN_CIRCLE: f64 = 5.0;
/// How wide and tall a key linked to an entity leaves its value.
const POINT_DIAMETER: f64 = 7.0;

/// The box of a map or a JSON element: its name and stereotype above the body, and its colours.
pub(super) struct DataBox {
    image: AbstractEntityImage,
    name: Box<dyn TextBlock>,
    stereo: Option<Box<dyn TextBlock>>,
    pub style: Style,
    url: Option<Url>,
    group: UGroup,
    round_corner: f64,
    minimum_width: f64,
    border_color: HColor,
    header_backcolor: HColor,
    backcolor: HColor,
    stroke: UStroke,
}

impl DataBox {
    /// `kind` is the element's style name, `map` or `json`; maps may have their own line colour.
    pub(super) fn new(entity: &Entity, diagram: &CucaDiagram, kind: SName) -> Self {
        let skin = diagram.skin();
        let stereotype = entity.stereotype.as_ref();
        let builder = skin.current_style_builder();
        let signature = |more: &[SName]| {
            let mut names = vec![SName::Root, SName::Element, SName::ObjectDiagram, kind];
            names.extend_from_slice(more);
            StyleSignature::of(&names).get_merged_style_with(&builder, stereotype)
        };
        let style = signature(&[]);
        let style_header = signature(&[SName::Header]);
        let name = entity.display.create0(
            &style_header.font_configuration(),
            HorizontalAlignment::Center,
            skin,
            0.0,
            CreoleMode::Full,
        );
        let stereo = stereotype
            .filter(|stereotype| !stereotype.label_double_comparator().is_empty())
            .filter(|_| diagram.show_portion(EntityPortion::Stereotype, entity.id()))
            .map(|stereotype| -> Box<dyn TextBlock> {
                Box::new(Display::create(stereotype.labels()).create0(
                    &skin.get_font_configuration(FontParam::ObjectStereotype, Some(stereotype)),
                    HorizontalAlignment::Center,
                    skin,
                    0.0,
                    CreoleMode::Full,
                ))
            });
        let colors = &entity.colors;
        let backcolor = colors.get(ColorType::Back).cloned();
        let header_backcolor = colors
            .get(ColorType::Header)
            .cloned()
            .or_else(|| backcolor.clone())
            .unwrap_or_else(|| style_header.value(PName::BackGroundColor).as_color());
        let line_color = colors.get(ColorType::Line).filter(|_| kind == SName::Map);
        Self {
            image: AbstractEntityImage::new(entity, diagram),
            name: Box::new(TextBlockMarged::new(
                name,
                ClockwiseTopRightBottomLeft::same(2.0),
            )),
            stereo,
            url: entity.url.clone(),
            group: entity_group(entity, diagram, "entity", entity.get_location()),
            round_corner: style.value(PName::RoundCorner).as_double(),
            minimum_width: style.value(PName::MinimumWidth).as_double(),
            border_color: line_color
                .cloned()
                .unwrap_or_else(|| style.value(PName::LineColor).as_color()),
            header_backcolor,
            backcolor: backcolor.unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            stroke: style.stroke(),
            style,
        }
    }

    fn title_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        name_and_stereotype_dimension(&*self.name, self.stereo.as_deref(), string_bounder)
    }

    /// The size of the box around a body of size `body`; an empty body of a JSON element takes some room.
    pub(super) fn dimension(
        &self,
        body: XDimension2D,
        empty_body_height: f64,
        string_bounder: &dyn StringBounder,
    ) -> XDimension2D {
        let title = self.title_dimension(string_bounder);
        let width = body
            .width
            .max(title.width + 2.0 * X_MARGIN_CIRCLE)
            .max(self.minimum_width);
        let body_height = if body.height == 0.0 {
            empty_body_height
        } else {
            body.height
        };
        XDimension2D::new(width, body_height + title.height)
    }

    /// Draws the box of size `total`, then the body as `draw_body` draws it below the title, given the
    /// whole width.
    pub(super) fn draw(
        &self,
        ug: &UGraphic,
        total: XDimension2D,
        draw_body: impl FnOnce(&UGraphic, f64),
    ) {
        let title = self.title_dimension(ug.string_bounder());
        let ug = ug
            .with_color(self.border_color.clone())
            .with_backcolor(self.backcolor.clone());
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        ug.start_group(&self.group);
        ug.with_stroke(self.stroke).draw(&UShape::Rectangle(
            URectangle::new(total.width, total.height).rounded(self.round_corner),
        ));
        if self.backcolor != self.header_backcolor {
            ug.with_backcolor(self.header_backcolor.clone())
                .with_stroke(self.stroke)
                .draw(&URectangle::new(total.width, title.height).half_rounded(self.round_corner));
        }
        let mut blocks: Vec<&dyn TextBlock> = Vec::new();
        if let Some(stereo) = &self.stereo {
            blocks.push(stereo.as_ref());
        }
        blocks.push(self.name.as_ref());
        draw_title(
            &ug,
            &blocks,
            total.width,
            title.height,
            HorizontalAlignment::Center,
        );
        let ug2 = ug.with_stencil_stroke(
            Rc::new(RectangleStencil { width: total.width }),
            self.stroke,
        );
        draw_body(&ug2.translated(0.0, title.height), total.width);
        if self.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }

    pub(super) fn backcolor(&self) -> HColor {
        self.image.get_backcolor()
    }
}

/// A key or value: creole text with room around it.
pub(super) fn entry_text(
    text: &str,
    font: &FontConfiguration,
    skin: &SkinParam,
    word_wrap: f64,
) -> Box<dyn TextBlock> {
    let block = Display::with_newlines(text).create0(
        font,
        HorizontalAlignment::Left,
        skin,
        word_wrap,
        CreoleMode::Full,
    );
    Box::new(TextBlockMarged::new(
        block,
        ClockwiseTopRightBottomLeft::margin1_margin2(2.0, 5.0),
    ))
}

pub(crate) struct EntityImageMap {
    data_box: DataBox,
    entries: TextBlockMap,
}

impl EntityImageMap {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let data_box = DataBox::new(entity, diagram, SName::Map);
        let Bodier::Map { map } = &entity.bodier else {
            unreachable!("maps have map bodies")
        };
        let style = &data_box.style;
        let entries = TextBlockMap::new(
            &style.font_configuration(),
            diagram.skin(),
            map,
            style.wrap_width(),
            style.horizontal_alignment().unwrap_or_default(),
        );
        Self { data_box, entries }
    }
}

impl TextBlock for EntityImageMap {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let entries = self.entries.calculate_dimension(string_bounder);
        self.data_box.dimension(entries, 0.0, string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let total = self.calculate_dimension(ug.string_bounder());
        self.data_box.draw(ug, total, |ug, width| {
            self.entries.draw_with_total_width(ug, width);
        });
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.data_box.backcolor())
    }
}

impl IEntityImage for EntityImageMap {}

/// A row: the key, and its value unless the key links to an entity.
struct MapRow {
    key: Box<dyn TextBlock>,
    value: Option<Box<dyn TextBlock>>,
}

impl MapRow {
    fn value_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.value
            .as_ref()
            .map_or(XDimension2D::new(POINT_DIAMETER, POINT_DIAMETER), |value| {
                value.calculate_dimension(string_bounder)
            })
    }

    fn height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.key
            .calculate_dimension(string_bounder)
            .height
            .max(self.value_dimension(string_bounder).height)
    }
}

/// The table of a map's keys and values (`TextBlockMap`).
struct TextBlockMap {
    rows: Vec<MapRow>,
    horizontal_alignment: HorizontalAlignment,
}

impl TextBlockMap {
    fn new(
        font: &FontConfiguration,
        skin: &SkinParam,
        map: &[(String, String)],
        word_wrap: f64,
        horizontal_alignment: HorizontalAlignment,
    ) -> Self {
        let rows = map
            .iter()
            .map(|(key, value)| {
                let key = if VisibilityModifier::is_visibility_character(key) {
                    key.chars().skip(1).collect()
                } else {
                    key.clone()
                };
                MapRow {
                    key: entry_text(&key, font, skin, word_wrap),
                    value: (value != "\0").then(|| entry_text(value, font, skin, word_wrap)),
                }
            })
            .collect();
        Self {
            rows,
            horizontal_alignment,
        }
    }

    fn width_col_a(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.rows
            .iter()
            .map(|row| row.key.calculate_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    fn width_col_b(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.rows
            .iter()
            .map(|row| row.value_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let height = self.rows.iter().map(|row| row.height(string_bounder)).sum();
        XDimension2D::new(
            self.width_col_a(string_bounder) + self.width_col_b(string_bounder),
            height,
        )
    }

    /// Draws the rows across `total_width`, the width of the whole map.
    fn draw_with_total_width(&self, ug: &UGraphic, total_width: f64) {
        let string_bounder = ug.string_bounder();
        let true_width = self
            .calculate_dimension(string_bounder)
            .width
            .max(total_width);
        let width_col_a = self.width_col_a(string_bounder);
        let mut y = 0.0;
        for row in &self.rows {
            let ugline = ug.translated(0.0, y);
            ugline.draw(&UShape::Line {
                dx: true_width,
                dy: 0.0,
            });
            let height_of_row = row.height(string_bounder);
            let key_width = row.key.calculate_dimension(string_bounder).width;
            match &row.value {
                None => {
                    let pos_col_a = self.horizontal_alignment.offset(true_width, key_width);
                    row.key.draw_u(&ugline.translated(pos_col_a, 0.0));
                }
                Some(value) => {
                    let pos_col_a = self.horizontal_alignment.offset(width_col_a, key_width);
                    row.key.draw_u(&ugline.translated(pos_col_a, 0.0));
                    value.draw_u(&ugline.translated(width_col_a, 0.0));
                    ugline.translated(width_col_a, 0.0).draw(&UShape::Line {
                        dx: 0.0,
                        dy: height_of_row,
                    });
                }
            }
            y += height_of_row;
        }
    }
}
