//! A state stereotyped `<<sdlreceive>>`: its name in a frame (PlantUML's `EntityImageState2`).

use std::rc::Rc;

use crate::abel::Entity;
use crate::creole::{CreoleMode, Display};
use crate::decoration::symbol::{Block, USymbols};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::blocks::{TextBlockMarged, TextBlockVertical};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;
use crate::style::{SName, Style, StyleSignature};
use crate::svek::{AbstractEntityImage, IEntityImage, ShapeType};

pub(crate) struct EntityImageState2 {
    base: AbstractEntityImage,
    url: Option<Url>,
    as_small: Box<dyn TextBlock>,
}

impl EntityImageState2 {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::State,
        ])
        .get_merged_style(&diagram.skin().current_style_builder());
        let alignment = diagram
            .skin()
            .default_text_alignment(HorizontalAlignment::Center);
        let desc: Block = Rc::new(body(&entity.display, entity, alignment, &style, diagram));
        let empty = || -> Block { Rc::new(TextBlockEmpty::default()) };
        let as_small = USymbols::FRAME.as_small(
            empty(),
            desc,
            empty(),
            style.symbol_context(&crate::color::Colors::default()),
            diagram.skin().stereotype_alignment(),
        );
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            url: entity.url.clone(),
            as_small,
        }
    }
}

/// The display as a body draws lines without separators (`BodyEnhanced1` over `MethodsOrFieldsArea`): each
/// line aligned on the widest, with room on both sides.
fn body(
    display: &Display,
    entity: &Entity,
    alignment: HorizontalAlignment,
    style: &Style,
    diagram: &CucaDiagram,
) -> impl TextBlock + 'static {
    let font = style.font_configuration_with(&entity.colors);
    let blocks = display
        .lines()
        .iter()
        .map(|line| -> Box<dyn TextBlock> {
            Box::new(Display::with_newlines(line).create0(
                font.clone(),
                alignment,
                diagram.skin(),
                style.wrap_width(),
                CreoleMode::Full,
            ))
        })
        .collect();
    TextBlockMarged::new(
        TextBlockVertical::new(blocks, alignment),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 6.0, 0.0, 6.0),
    )
}

impl TextBlock for EntityImageState2 {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.as_small.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        self.as_small.draw_u(ug);
        if self.url.is_some() {
            ug.close_url();
        }
    }
}

impl IEntityImage for EntityImageState2 {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}
