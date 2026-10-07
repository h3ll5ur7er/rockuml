//! Tiles that span participants rather than belong to one: dividers, delays, vertical space, references
//! and page breaks (PlantUML's `DividerTile`, `DelayTile`, `HSpaceTile`, `ReferenceTile` and
//! `NewpageTile`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::components;
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, Labelled, NotePosition, Reference};
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, Component, Context2D};
use crate::style::StyleBuilder;

/// `== text ==` across the whole diagram.
pub(super) struct DividerTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    y_gauge: YGauge,
    component: Box<dyn Component>,
}

impl<'a> DividerTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        divider: &'a Labelled,
        current_y: &YGauge,
    ) -> Self {
        let component = components::divider(divider, arguments.diagram.skin());
        let height = component.preferred_height(arguments.string_bounder());
        Self {
            y_gauge: YGauge::create(&current_y.max, height),
            arguments,
            event,
            component,
        }
    }
}

impl<'a> Tile<'a> for DividerTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn preferred_height(&self) -> f64 {
        self.component
            .preferred_height(self.arguments.string_bounder())
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.arguments.x_origin.clone()
    }

    fn max_x(&self) -> Real {
        let width = self
            .component
            .preferred_width(self.arguments.string_bounder());
        self.arguments.x_origin.add_fixed(width)
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let height = self.preferred_height();
        let arguments = &self.arguments;
        let width = arguments.border2() - arguments.border1() - arguments.x_origin.current_value();
        self.component.draw_u(
            &ug.translated(arguments.border1(), 0.0),
            &Area::new(width, height),
            context,
        );
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }
}

/// `...` or `...text...`: the lifelines go dotted for a while.
pub(super) struct DelayTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    y_gauge: YGauge,
    component: Box<dyn Component>,
    middle: OnceCell<Real>,
}

impl<'a> DelayTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        delay: &'a Labelled,
        current_y: &YGauge,
    ) -> Self {
        let component = components::delay_text(delay, arguments.diagram.skin());
        let height = component.preferred_height(arguments.string_bounder());
        Self {
            y_gauge: YGauge::create(&current_y.max, height),
            arguments,
            event,
            component,
            middle: OnceCell::new(),
        }
    }

    /// Halfway between the first and the last participant.
    fn middle(&self) -> &Real {
        self.middle.get_or_init(|| {
            let string_bounder = self.arguments.string_bounder();
            let living_spaces = &self.arguments.living_spaces;
            Real::middle(
                &living_spaces.first().pos_c(string_bounder),
                &living_spaces.last().pos_c(string_bounder),
            )
        })
    }

    fn width(&self) -> f64 {
        self.component
            .preferred_width(self.arguments.string_bounder())
    }
}

impl<'a> Tile<'a> for DelayTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn preferred_height(&self) -> f64 {
        self.component
            .preferred_height(self.arguments.string_bounder())
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.middle().add_fixed(-self.width() / 2.0)
    }

    fn max_x(&self) -> Real {
        self.middle().add_fixed(self.width() / 2.0)
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let height = self.preferred_height();
        let y = self.y_gauge.min.current_value();
        self.arguments.living_spaces.delay_on(y, height);
        let ug = ug.translated(self.min_x().current_value(), y);
        self.component
            .draw_u(&ug, &Area::new(self.width(), height), context);
    }
}

/// `|||` or `||45||`: vertical space.
pub(super) struct HSpaceTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    pixels: i32,
    y_gauge: YGauge,
}

impl<'a> HSpaceTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        pixels: i32,
        current_y: &YGauge,
    ) -> Self {
        Self {
            y_gauge: YGauge::create_propagating(current_y, f64::from(pixels)),
            arguments,
            event,
            pixels,
        }
    }
}

impl<'a> Tile<'a> for HSpaceTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn preferred_height(&self) -> f64 {
        f64::from(self.pixels)
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.arguments.x_origin.clone()
    }

    fn max_x(&self) -> Real {
        self.arguments.x_origin.add_fixed(10.0)
    }

    fn draw_u(&self, _ug: &UGraphic, _context: Context2D) {}
}

/// `ref over A, B : text`, with notes on its sides.
pub(super) struct ReferenceTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    reference: &'a Reference,
    y_gauge: YGauge,
    component: Box<dyn Component>,
    note_left: Option<Box<dyn Component>>,
    note_right: Option<Box<dyn Component>>,
    /// The frame's left and right edges, decided when first needed.
    edges: OnceCell<(Real, Real)>,
}

impl<'a> ReferenceTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        reference: &'a Reference,
        current_y: &YGauge,
    ) -> Self {
        let component = components::reference(reference, arguments.diagram.skin());
        let height = component.preferred_height(arguments.string_bounder());
        let mut note_left = None;
        let mut note_right = None;
        for note in &reference.notes {
            let component = components::note(arguments.diagram, note, false, false);
            if note.position == NotePosition::Right {
                note_right = Some(component);
            } else {
                note_left = Some(component);
            }
        }
        Self {
            y_gauge: YGauge::create(&current_y.max, height),
            arguments,
            event,
            reference,
            component,
            note_left,
            note_right,
            edges: OnceCell::new(),
        }
    }

    /// The left edge of the leftmost participant and the right edge of the rightmost, pushed apart to fit the
    /// frame.
    fn edges(&self) -> &(Real, Real) {
        self.edges.get_or_init(|| {
            let string_bounder = self.arguments.string_bounder();
            let mut first: Option<Real> = None;
            let mut last: Option<Real> = None;
            for &participant in &self.reference.participants {
                let living_space = self.arguments.living_space(participant);
                let center = living_space.pos_c(string_bounder).current_value();
                if first
                    .as_ref()
                    .is_none_or(|first| center < first.current_value())
                {
                    first = Some(living_space.pos_b().clone());
                }
                if last
                    .as_ref()
                    .is_none_or(|last| center > last.current_value())
                {
                    last = Some(living_space.pos_d(string_bounder));
                }
            }
            let first = first.expect("a reference covers participants");
            let mut last = last.expect("a reference covers participants");
            if self.reference.participants.len() == 1 {
                last = last.add_at_least(0.0);
            }
            let width = self.component.preferred_width(string_bounder);
            last.ensure_bigger_than(&first.add_fixed(width));
            (first, last)
        })
    }

    fn note_width(&self, note: Option<&dyn Component>) -> Option<f64> {
        note.map(|note| note.preferred_width(self.arguments.string_bounder()))
    }
}

impl<'a> Tile<'a> for ReferenceTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn preferred_height(&self) -> f64 {
        self.component
            .preferred_height(self.arguments.string_bounder())
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        let first = &self.edges().0;
        match self.note_width(self.note_left.as_deref()) {
            Some(width) => first.add_fixed(-width),
            None => first.clone(),
        }
    }

    fn max_x(&self) -> Real {
        let last = &self.edges().1;
        match self.note_width(self.note_right.as_deref()) {
            Some(width) => last.add_fixed(width),
            None => last.clone(),
        }
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let string_bounder = ug.string_bounder();
        let (first, last) = self.edges();
        let (first, last) = (first.current_value(), last.current_value());
        let height = self.preferred_height();
        self.component.draw_u(
            &ug.translated(first, 0.0),
            &Area::new(last - first, height),
            context,
        );
        if let Some(note) = &self.note_left {
            let dimension = note.preferred_dimension(string_bounder);
            note.draw_u(
                &ug.translated(first - dimension.width, 0.0),
                &Area::new(dimension.width, dimension.height),
                context,
            );
        }
        if let Some(note) = &self.note_right {
            let dimension = note.preferred_dimension(string_bounder);
            note.draw_u(
                &ug.translated(last, 0.0),
                &Area::new(dimension.width, dimension.height),
                context,
            );
        }
    }

    fn stable_min_x(&self) -> Vec<Real> {
        vec![self.drawn_min_x()]
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }
}

/// `newpage`: the line where one page ends and the next starts.
pub(super) struct NewpageTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    y_gauge: YGauge,
    component: Box<dyn Component>,
}

/// Space above and below the line.
const NEWPAGE_MARGIN_Y: f64 = 10.0;

impl<'a> NewpageTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        style_builder: &StyleBuilder,
        current_y: &YGauge,
    ) -> Self {
        let component = components::newpage(style_builder);
        let height =
            component.preferred_height(arguments.string_bounder()) + 2.0 * NEWPAGE_MARGIN_Y;
        Self {
            y_gauge: YGauge::create(&current_y.max, height),
            arguments,
            event,
            component,
        }
    }
}

impl<'a> Tile<'a> for NewpageTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        0.0
    }

    fn preferred_height(&self) -> f64 {
        self.component
            .preferred_height(self.arguments.string_bounder())
            + 2.0 * NEWPAGE_MARGIN_Y
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.arguments.x_origin.clone()
    }

    fn max_x(&self) -> Real {
        self.arguments.x_origin.clone()
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        if context.is_background {
            return;
        }
        let arguments = &self.arguments;
        let ug = ug.translated(
            arguments.border1(),
            self.y_gauge.min.current_value() + NEWPAGE_MARGIN_Y,
        );
        let width = arguments.border2() - arguments.border1() - arguments.x_origin.current_value();
        let height = self.component.preferred_height(arguments.string_bounder());
        self.component
            .draw_u(&ug, &Area::new(width, height), context);
    }

    fn as_newpage(&self) -> Option<&NewpageTile<'a>> {
        Some(self)
    }
}
