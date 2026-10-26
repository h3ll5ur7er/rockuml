//! The elements of a BPM diagram, drawn by the activity diagram's tiles with red stubs on their connected
//! sides (PlantUML's `BpmElement` and `BpmElementType`).

use std::rc::Rc;

use super::grid::{Connections, Where};
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::ftile::vertical::{FtileBox, FtileCircleStart, FtileDiamond};
use crate::ftile::{BoxStyle, Ftile};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::style::{SName, StyleSignature};

/// PlantUML's `END` is never made, so it is left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BpmElementType {
    Start,
    Merge,
    DockedEvent,
}

#[derive(Clone, Debug)]
pub(super) struct BpmElement {
    pub(super) id: Option<String>,
    kind: BpmElementType,
    label: Option<String>,
    pub(super) connections: Connections,
}

impl BpmElement {
    pub(super) fn new(id: Option<String>, kind: BpmElementType, label: Option<String>) -> Self {
        Self {
            id,
            kind,
            label,
            connections: Connections::default(),
        }
    }

    /// The element's tile with its stubs (`toTextBlock`).
    pub(super) fn to_text_block(&self, skin_param: &Rc<SkinParam>) -> Box<dyn TextBlock> {
        Box::new(Stubbed {
            raw: self.to_ftile(skin_param),
            connections: self.connections,
        })
    }

    /// `toTextBlockInternal`.
    fn to_ftile(&self, skin_param: &Rc<SkinParam>) -> Box<dyn Ftile> {
        match self.kind {
            BpmElementType::Start => {
                let style = StyleSignature::of(&[
                    SName::Root,
                    SName::Element,
                    SName::ActivityDiagram,
                    SName::Circle,
                ])
                .get_merged_style(&skin_param.current_style_builder());
                Box::new(FtileCircleStart::new(
                    skin_param.clone(),
                    None,
                    &style,
                    &Colors::default(),
                ))
            }
            // The defaults of `ColorParam.activityBackground` and `activityBorder`, which no `skinparam`
            // reaches: the diagram draws with a skin of its own.
            BpmElementType::Merge => Box::new(FtileDiamond::new(
                skin_param.clone(),
                HColor::MY_YELLOW,
                HColor::MY_RED,
                None,
            )),
            BpmElementType::DockedEvent => Box::new(FtileBox::create(
                skin_param.clone(),
                &Colors::default(),
                &Display::with_newlines(self.label.as_deref().unwrap_or_default()),
                None,
                BoxStyle::Plain,
                None,
                &skin_param.current_style_builder(),
            )),
        }
    }
}

/// A tile, with a red stub on each side a connector leaves by.
struct Stubbed {
    raw: Box<dyn Ftile>,
    connections: Connections,
}

impl Stubbed {
    fn draw_line(ug: &UGraphic, w: Where, total: XDimension2D) {
        let (width, height) = (total.width, total.height);
        let (x, y, line) = match w {
            Where::West => (-10.0, height / 2.0, hline(10.0)),
            Where::East => (width, height / 2.0, hline(10.0)),
            Where::North => (width / 2.0, -10.0, vline(10.0)),
            Where::South => (width / 2.0, height, vline(10.0)),
        };
        ug.translated(x, y).draw(&line);
    }
}

impl TextBlock for Stubbed {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.raw.calculate_dimension(string_bounder).dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.raw.draw_u(ug);
        let ug = ug.with_color(HColor::RED);
        let total = self.calculate_dimension(ug.string_bounder());
        for w in Where::VALUES {
            if self.connections.have(w) {
                Self::draw_line(&ug, w, total);
            }
        }
    }
}

pub(super) fn hline(dx: f64) -> UShape {
    UShape::Line { dx, dy: 0.0 }
}

pub(super) fn vline(dy: f64) -> UShape {
    UShape::Line { dx: 0.0, dy }
}
