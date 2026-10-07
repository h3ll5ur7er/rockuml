//! The tips on one side of an entity, each a note pointing at its member (PlantUML's `EntityImageTips`).

use std::rc::Rc;

use super::opale::Opale;
use crate::abel::{Direction, Entity, Position};
use crate::color::ColorType;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{XDimension2D, XPoint2D, XRectangle2D};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::body::enhanced_text;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, ShapeType};

/// The space between two tips.
const Y_SPACING: f64 = 10.0;

pub(crate) struct EntityImageTips {
    base: AbstractEntityImage,
    position: Position,
    /// Each member's tip, in the order members were first given one.
    opales: Vec<(String, Opale<'static>)>,
}

impl EntityImageTips {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let base = AbstractEntityImage::new(entity, diagram);
        let skin = diagram.skin();
        let style_builder = skin.current_style_builder();
        // Each tip keeps its own stereotype and colours, so each has its own style.
        let opales = entity
            .get_tips()
            .iter()
            .map(|(member, tip)| {
                let signature = StyleSignature::of(&[
                    SName::Root,
                    SName::Element,
                    base.get_style_name(),
                    SName::Note,
                ])
                .with_tobechanged(tip.stereotype.as_ref());
                let style = style_builder
                    .merged_style(&signature)
                    .expect("the skin styles notes");
                let note_background_color = tip
                    .colors
                    .get(ColorType::Back)
                    .cloned()
                    .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
                let text_block = enhanced_text(
                    &tip.display,
                    style.font_configuration(),
                    HorizontalAlignment::Left,
                    &style,
                    skin,
                );
                let opale = Opale::new(
                    style.value(PName::LineColor).as_color(),
                    note_background_color,
                    text_block,
                    style.stroke(),
                    0.0,
                );
                (member.clone(), opale)
            })
            .collect();
        let position = if entity.get_name(diagram).ends_with(Position::Right.name()) {
            Position::Right
        } else {
            Position::Left
        };
        Self {
            base,
            position,
            opales,
        }
    }

    /// Draws the tips, laid out at `position_me`, each pointing at its member in the entity laid out at
    /// `position_other` (both top left corners). `inner_position` finds where that entity draws a member;
    /// drawing stops at the first member it cannot find (`drawU`).
    pub(crate) fn draw_tips(
        &self,
        ug: &UGraphic,
        position_me: XPoint2D,
        position_other: XPoint2D,
        inner_position: &dyn Fn(&str) -> Option<XRectangle2D>,
    ) {
        let string_bounder = ug.string_bounder();
        let mut direction = self.position.reverse_direction();
        let mut ug = ug.clone();
        let mut height = 0.0;
        for (member, opale) in &self.opales {
            let Some(member_position) = inner_position(member) else {
                return;
            };
            let dim = opale.calculate_dimension(string_bounder);
            let pp1 = XPoint2D::new(0.0, dim.height / 2.0);
            let mut x = position_other.x - position_me.x;
            if direction == Direction::Right && x < 0.0 {
                direction = direction.get_inv();
            }
            if direction == Direction::Left {
                x += member_position.get_max_x();
            } else {
                x += member_position.get_min_x();
            }
            let y = position_other.y - position_me.y - height + member_position.get_center_y();
            let pp2 = XPoint2D::new(x, y);
            opale.draw_u(
                &ug.with_stencil(Rc::new(RectangleStencil { width: dim.width })),
                direction,
                pp1,
                pp2,
            );
            ug = ug.translated(0.0, dim.height + Y_SPACING);
            height += dim.height + Y_SPACING;
        }
    }
}

impl TextBlock for EntityImageTips {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.opales
            .iter()
            .map(|(_, opale)| opale.calculate_dimension(string_bounder))
            .fold(XDimension2D::default(), |result, dimension| {
                XDimension2D::new(
                    result.width.max(dimension.width),
                    result.height + dimension.height + Y_SPACING,
                )
            })
    }

    /// Tips are only drawn in a layout, next to their entity's members (see `draw_tips`).
    fn draw_u(&self, _ug: &UGraphic) {}

    fn backcolor(&self) -> Option<crate::color::HColor> {
        Some(self.base.get_backcolor())
    }
}

impl IEntityImage for EntityImageTips {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}
