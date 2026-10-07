//! A history state, a circle around `H`, or `H*` for a deep history (PlantUML's `EntityImagePseudoState`
//! and `EntityImageDeepHistory`).

use crate::abel::Entity;
use crate::color::HColor;
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, ShapeType};

const SIZE: f64 = 22.0;

pub(crate) struct EntityImagePseudoState {
    base: AbstractEntityImage,
    desc: SheetBlock2,
    border_color: HColor,
    background_color: HColor,
    stroke: UStroke,
}

impl EntityImagePseudoState {
    /// A shallow history, `H`.
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self::with_text(entity, diagram, "H")
    }

    /// A deep history, `H*` (`EntityImageDeepHistory`).
    pub(crate) fn deep_history(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self::with_text(entity, diagram, "H*")
    }

    fn with_text(entity: &Entity, diagram: &CucaDiagram, history_text: &str) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Diamond,
        ])
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            entity.stereotype.as_ref(),
        );
        let desc = Display::create([history_text]).create0(
            style.font_configuration(),
            HorizontalAlignment::Center,
            diagram.skin(),
            0.0,
            CreoleMode::Full,
        );
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            desc,
            border_color: style.value(PName::LineColor).as_color(),
            background_color: style.value(PName::BackGroundColor).as_color(),
            stroke: style.stroke(),
        }
    }
}

impl TextBlock for EntityImagePseudoState {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(SIZE, SIZE)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug
            .with_stroke(self.stroke)
            .with_backcolor(self.background_color.clone())
            .with_color(self.border_color.clone());
        ug.draw(&UShape::Ellipse(UEllipse::new(SIZE, SIZE)));
        let dim_desc = self.desc.calculate_dimension(ug.string_bounder());
        let x = (SIZE - dim_desc.width) / 2.0;
        let y = (SIZE - dim_desc.height) / 2.0;
        self.desc.draw_u(&ug.translated(x, y));
    }
}

impl IEntityImage for EntityImagePseudoState {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Circle
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}
