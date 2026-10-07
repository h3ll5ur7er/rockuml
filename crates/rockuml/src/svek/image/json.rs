//! A JSON element: a box with its name above its data, objects as tables of names and values and arrays
//! as rows (PlantUML's `EntityImageJson` and `TextBlockCucaJSon`).

use super::map::{DataBox, entry_text};
use crate::abel::Entity;
use crate::color::HColor;
use crate::cucadiagram::Bodier;
use crate::diagram::cuca::CucaDiagram;
use crate::json::JsonValue;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::TextBlock;
use crate::skin::SkinParam;
use crate::style::SName;
use crate::svek::{IEntityImage, ShapeType};

/// How tall a JSON element without data is.
const MARGIN_EMPTY_FIELDS_OR_METHOD: f64 = 13.0;

pub(crate) struct EntityImageJson {
    data_box: DataBox,
    entries: JsonBlock,
}

impl EntityImageJson {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let data_box = DataBox::new(entity, diagram, SName::Json);
        let Bodier::Json { json: Some(json) } = entity.get_bodier() else {
            unreachable!("JSON elements have their data")
        };
        let style = &data_box.style;
        let entries = JsonBlock::new(
            json,
            &style.font_configuration(),
            diagram.skin(),
            style.wrap_width(),
        );
        Self { data_box, entries }
    }
}

impl TextBlock for EntityImageJson {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let entries = self.entries.calculate_dimension(string_bounder);
        self.data_box
            .dimension(entries, MARGIN_EMPTY_FIELDS_OR_METHOD, string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let total = self.calculate_dimension(ug.string_bounder());
        self.data_box.draw(ug, total, |ug, width| {
            self.entries.draw(ug, width);
        });
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.data_box.backcolor())
    }
}

impl IEntityImage for EntityImageJson {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::RectangleHtmlForPorts
    }

    fn is_hidden(&self) -> bool {
        self.data_box.is_hidden()
    }
}

/// A JSON value drawn (`TextBlockCucaJSon`'s blocks).
enum JsonBlock {
    Text(Box<dyn TextBlock>),
    /// Each member's name and value.
    Object(Vec<(Box<dyn TextBlock>, JsonBlock)>),
    Array(Vec<JsonBlock>),
}

impl JsonBlock {
    fn new(value: &JsonValue, font: &FontConfiguration, skin: &SkinParam, word_wrap: f64) -> Self {
        let text = |text: &str| entry_text(text, font, skin, word_wrap);
        match value {
            JsonValue::String(string) => Self::Text(text(string)),
            JsonValue::Array(values) => Self::Array(
                values
                    .iter()
                    .map(|value| Self::new(value, font, skin, word_wrap))
                    .collect(),
            ),
            JsonValue::Object(object) => Self::Object(
                object
                    .members()
                    .map(|(name, value)| (text(name), Self::new(value, font, skin, word_wrap)))
                    .collect(),
            ),
            other => Self::Text(text(&other.to_string())),
        }
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        match self {
            Self::Text(block) => block.calculate_dimension(string_bounder),
            Self::Object(members) => {
                let (width1, width2, height) = object_dimension(members, string_bounder);
                XDimension2D::new(width1 + width2, height)
            }
            Self::Array(elements) => elements
                .iter()
                .map(|element| element.calculate_dimension(string_bounder))
                .fold(XDimension2D::default(), XDimension2D::merge_top_bottom),
        }
    }

    /// Draws the value, its rows spanning `total_width`.
    fn draw(&self, ug: &UGraphic, total_width: f64) {
        let string_bounder = ug.string_bounder();
        let hline = UShape::Line {
            dx: total_width,
            dy: 0.0,
        };
        match self {
            Self::Text(block) => block.draw_u(ug),
            Self::Object(members) => {
                let (width1, _, height) = object_dimension(members, string_bounder);
                ug.translated(width1, 0.0).draw(&UShape::Line { dx: 0.0, dy: height });
                let mut y = 0.0;
                for (name, value) in members {
                    let row = ug.translated(0.0, y);
                    row.draw(&hline);
                    name.draw_u(&row);
                    value.draw(&row.translated(width1, 0.0), total_width - width1);
                    y += name
                        .calculate_dimension(string_bounder)
                        .height
                        .max(value.calculate_dimension(string_bounder).height);
                }
            }
            Self::Array(elements) => {
                let mut y = 0.0;
                for (index, element) in elements.iter().enumerate() {
                    let row = ug.translated(0.0, y);
                    if index > 0 {
                        row.draw(&hline);
                    }
                    element.draw(&row, total_width);
                    y += element.calculate_dimension(string_bounder).height;
                }
            }
        }
    }
}

/// The widest name, the widest value, and the height of the rows of an object.
fn object_dimension(
    members: &[(Box<dyn TextBlock>, JsonBlock)],
    string_bounder: &dyn StringBounder,
) -> (f64, f64, f64) {
    members.iter().fold((0.0, 0.0, 0.0), |(width1, width2, height), (name, value)| {
        let name = name.calculate_dimension(string_bounder);
        let value = value.calculate_dimension(string_bounder);
        (
            f64::max(width1, name.width),
            f64::max(width2, value.width),
            height + name.height.max(value.height),
        )
    })
}
