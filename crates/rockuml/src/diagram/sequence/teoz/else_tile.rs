//! Where an `else` starts in a group: a dashed line across the group's frame (PlantUML's `ElseTile`).

use std::rc::Rc;

use super::components;
use super::grouping_tile::GroupingFrame;
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, GroupingLeaf};
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, Component, Context2D};

pub(super) struct ElseTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    parent: Rc<GroupingFrame>,
    y_gauge: YGauge,
    component: Box<dyn Component>,
}

impl<'a> ElseTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        leaf: &'a GroupingLeaf,
        parent: Rc<GroupingFrame>,
        current_y: &YGauge,
    ) -> Self {
        let component = components::grouping_else(leaf);
        let height = component
            .preferred_dimension(arguments.string_bounder())
            .height;
        Self {
            y_gauge: YGauge::create(&current_y.max, height),
            arguments,
            event,
            parent,
            component,
        }
    }
}

impl<'a> Tile<'a> for ElseTile<'a> {
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
            .preferred_dimension(self.arguments.string_bounder())
            .height
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.parent.min_x()
    }

    fn max_x(&self) -> Real {
        let width = self
            .component
            .preferred_dimension(self.arguments.string_bounder())
            .width;
        self.min_x().add_fixed(width)
    }

    /// The line spans the group's frame, not the room the group keeps for the notes beside it.
    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let dimension = self.component.preferred_dimension(ug.string_bounder());
        let min = self.parent.min().current_value();
        let max = self.parent.max().current_value();
        self.component.draw_u(
            &ug.translated(min, 0.0),
            &Area::new(max - min, dimension.height),
            context,
        );
    }
}
