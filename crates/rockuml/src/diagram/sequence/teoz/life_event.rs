//! `activate`, `deactivate`, `destroy` and `create`: they only mark the lifeline, except a destroy without a
//! message, which draws its cross (PlantUML's `LifeEventTile`).

use std::rc::Rc;

use super::communication::LIVE_DELTA_SIZE;
use super::components;
use super::living_space::{EventsHistoryMode, LivingSpace};
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, LifeEvent, LifeEventType};
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, Context2D};

pub(super) struct LifeEventTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    life_event: &'a LifeEvent,
    y_gauge: YGauge,
}

impl<'a> LifeEventTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        life_event: &'a LifeEvent,
        current_y: &YGauge,
    ) -> Self {
        let mut tile = Self {
            arguments,
            event,
            life_event,
            y_gauge: current_y.clone(),
        };
        tile.y_gauge = YGauge::create_propagating(current_y, tile.preferred_height());
        tile
    }

    fn living_space(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.life_event.participant)
    }

    fn is_destroy_without_message(&self) -> bool {
        self.life_event.message.is_none() && self.life_event.kind == LifeEventType::Destroy
    }

    fn level(&self) -> usize {
        self.living_space()
            .level_at(self.event, EventsHistoryMode::IgnoreFutureActivate)
    }
}

impl<'a> Tile<'a> for LifeEventTile<'a> {
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
        if self.is_destroy_without_message() {
            return components::destroy(self.arguments.diagram)
                .preferred_dimension(self.arguments.string_bounder())
                .height;
        }
        0.0
    }

    fn on_gauge_resolved(&self) {
        let y = self.y_gauge.min.current_value();
        self.living_space().add_step_for_livebox(self.event, y);
        if self.life_event.kind == LifeEventType::Destroy {
            self.living_space().go_destroy(y);
        }
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        let adjustment = if self.level() > 0 {
            LIVE_DELTA_SIZE
        } else {
            0.0
        };
        self.living_space()
            .pos_c(self.arguments.string_bounder())
            .add_fixed(-adjustment)
    }

    fn max_x(&self) -> Real {
        let adjustment = self.level() as f64 * LIVE_DELTA_SIZE;
        self.living_space()
            .pos_c(self.arguments.string_bounder())
            .add_fixed(adjustment)
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        if !self.is_destroy_without_message() {
            return;
        }
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let cross = components::destroy(self.arguments.diagram);
        let dimension = cross.preferred_dimension(ug.string_bounder());
        let x = self
            .living_space()
            .pos_c(ug.string_bounder())
            .current_value();
        cross.draw_u(
            &ug.translated(x - dimension.width / 2.0, 0.0),
            &Area::default(),
            context,
        );
    }

    fn stable_min_x(&self) -> Vec<Real> {
        vec![self.drawn_min_x()]
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }

    fn is_life_event(&self) -> bool {
        true
    }
}
