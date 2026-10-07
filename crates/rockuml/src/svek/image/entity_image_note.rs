//! A note of its own or on an entity (PlantUML's `EntityImageNote`).

use std::rc::Rc;

use super::opale::{self, MARGIN_X1, MARGIN_X2, MARGIN_Y, Opale};
use crate::abel::{Entity, EntityId, LinkId};
use crate::color::{ColorType, HColor};
use crate::diagram::cuca::CucaDiagram;
use crate::direction::Direction;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::UShape;
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::body::enhanced_text;
use crate::skin::component::TextBlockEmpty;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, Style, StyleBuilder, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, LayoutContext};

/// Where the one link of a note drawn as a callout runs once laid out, instead of being drawn itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OpaleLink {
    /// The link's ends, where the diagram is drawn.
    pub start: XPoint2D,
    pub end: XPoint2D,
    /// The note's top left corner.
    pub node_min: XPoint2D,
    /// How far the outline of the entity at the other end pulls `end` (`other.getMagneticBorder()`).
    pub other_force: UTranslate,
}

pub(crate) struct EntityImageNote {
    base: AbstractEntityImage,
    group: UGroup,
    url: Option<Url>,
    note_background_color: HColor,
    border_color: HColor,
    stroke: UStroke,
    round_corner: f64,
    text_block: Box<dyn TextBlock>,
    /// The link drawn as a callout, and the entity at its other end.
    opale_link: Option<(LinkId, EntityId)>,
}

impl EntityImageNote {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let base = AbstractEntityImage::new(entity, diagram);
        let skin = diagram.skin();
        let style = note_style(
            &skin.current_style_builder(),
            base.get_style_name(),
            base.get_stereo(),
        );
        let note_background_color = entity
            .colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        let display = &entity.display;
        let text_block: Box<dyn TextBlock> = if display.is_single_empty_line() {
            Box::new(TextBlockEmpty::default())
        } else {
            enhanced_text(
                display,
                style.font_configuration(),
                style
                    .horizontal_alignment()
                    .unwrap_or(HorizontalAlignment::Left),
                &style,
                skin,
            )
        };
        let name = entity.get_name(diagram);
        let mut group = UGroup::at(entity.get_location());
        group.put(UGroupType::Class, "entity");
        group.put(UGroupType::Id, &format!("entity_{name}"));
        group.put(UGroupType::DataEntity, name);
        group.put(UGroupType::DataUid, entity.get_uid());
        group.put(
            UGroupType::DataQualifiedName,
            diagram.quark(entity.get_quark()).get_qualified_name(),
        );
        Self {
            group,
            url: entity.url.clone(),
            note_background_color,
            border_color: style.value(PName::LineColor).as_color(),
            stroke: style.stroke(),
            round_corner: skin.get_round_corner(),
            text_block,
            opale_link: None,
            base,
        }
    }

    fn get_text_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(string_bounder).width + MARGIN_X1 + MARGIN_X2
    }

    fn get_text_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(string_bounder).height + 2.0 * MARGIN_Y
    }

    /// Draws the note, as a callout along `opale_link` if it has one.
    pub(crate) fn draw_with(&self, ug: &UGraphic, opale_link: Option<OpaleLink>) {
        ug.start_group(&self.group);
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        let width = self.calculate_dimension(ug.string_bounder()).width;
        let ug2 = ug.with_stencil(Rc::new(RectangleStencil { width }));
        match opale_link {
            Some(link) => self.draw_opale(&ug2, link),
            None => self.draw_normal(&ug2),
        }
        if self.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }

    fn draw_normal(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let polygon = opale::get_polygon_normal(
            self.get_text_width(string_bounder),
            self.get_text_height(string_bounder),
            self.round_corner,
        );
        let ug = ug
            .with_backcolor(self.note_background_color.clone())
            .with_color(self.border_color.clone());
        ug.with_stroke(self.stroke).draw(&UShape::Path(polygon));
        ug.draw(&UShape::Path(opale::get_corner(
            self.get_text_width(string_bounder),
            self.round_corner,
        )));
        self.text_block.draw_u(&ug.translated(MARGIN_X1, MARGIN_Y));
    }

    fn draw_opale(&self, ug: &UGraphic, link: OpaleLink) {
        let string_bounder = ug.string_bounder();
        let to_note = |point: XPoint2D| {
            UTranslate::new(-link.node_min.x, -link.node_min.y).get_translated(point)
        };
        let start_point = to_note(link.start);
        let end_point = to_note(link.end);
        let force1 = self.magnetic_border_force_at(string_bounder, link.start);
        let force2 = link.other_force;
        let text_width = self.get_text_width(string_bounder);
        let text_height = self.get_text_height(string_bounder);
        let center = XPoint2D::new(text_width / 2.0, text_height / 2.0);
        let mut pp1 = force2.get_translated(start_point);
        let mut pp2 = force1.get_translated(end_point);
        if pp1.distance(center) < pp2.distance(center) {
            pp1 = force1.get_translated(end_point);
            pp2 = force2.get_translated(start_point);
        }
        let strategy = get_opale_strategy(text_width, text_height, pp2);
        let opale = Opale::new(
            self.border_color.clone(),
            self.note_background_color.clone(),
            Box::new(&*self.text_block),
            self.stroke,
            self.round_corner,
        );
        opale.draw_u(&ug.with_stroke(self.stroke), strategy, pp2, pp1);
    }
}

/// The style of a note in a diagram styled `style_name` (`getStyleSignature().getMergedStyle`).
pub(super) fn note_style(
    builder: &StyleBuilder,
    style_name: SName,
    stereotype: Option<&Stereotype>,
) -> Style {
    StyleSignature::of(&[SName::Root, SName::Element, style_name, SName::Note])
        .get_merged_style_with(builder, stereotype)
}

/// The side of a `width` by `height` note nearest to `pt`.
fn get_opale_strategy(width: f64, height: f64, pt: XPoint2D) -> Direction {
    let d1 = (width - pt.x).abs();
    let d2 = (height - pt.y).abs();
    let d3 = pt.x.abs();
    let d4 = pt.y.abs();
    if d3 <= d1 && d3 <= d2 && d3 <= d4 {
        Direction::Left
    } else if d1 <= d2 && d1 <= d3 && d1 <= d4 {
        Direction::Right
    } else if d4 <= d1 && d4 <= d2 && d4 <= d3 {
        Direction::Up
    } else {
        Direction::Down
    }
}

impl TextBlock for EntityImageNote {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.get_text_width(string_bounder),
            self.get_text_height(string_bounder),
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.draw_with(ug, None);
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.base.get_backcolor())
    }
}

impl IEntityImage for EntityImageNote {
    fn set_opale_link(&mut self, link: LinkId, other: EntityId) {
        self.opale_link = Some((link, other));
    }

    fn draw_u_in_layout(&self, ug: &UGraphic, layout: &LayoutContext<'_>) {
        let opale_link = self.opale_link.map(|(link, other)| {
            let edge = layout
                .get_smetana_edge(link)
                .expect("a note's callout follows its link's route");
            let start = edge.get_start_point().expect("routed links have ends");
            let end = edge.get_end_point().expect("routed links have ends");
            let node = layout.get_node(self.base.get_entity());
            OpaleLink {
                start,
                end,
                node_min: XPoint2D::new(node.get_min_x(), node.get_min_y()),
                other_force: layout
                    .get_node(other)
                    .get_magnetic_border_force_at(ug.string_bounder(), end),
            }
        });
        self.draw_with(ug, opale_link);
    }
}
