//! Messages from a participant to itself (PlantUML's `CommunicationTileSelf`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::communication::LIVE_DELTA_SIZE;
use super::living_space::{EventsHistoryMode, LivingSpace};
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, Message};
use crate::diagram::sequence::styles::message_style;
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, ArrowComponent, Component, Context2D};
use crate::skin::rose::self_arrow::ComponentRoseSelfArrow;
use crate::skin::rose::{MessageLabel, create_component_self_arrow};

pub(super) struct CommunicationTileSelf<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    message: &'a Message,
    y_gauge: YGauge,
    component: OnceCell<ComponentRoseSelfArrow>,
}

impl<'a> CommunicationTileSelf<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        message: &'a Message,
        current_y: &YGauge,
    ) -> Self {
        let mut tile = Self {
            arguments,
            event,
            message,
            y_gauge: current_y.clone(),
            component: OnceCell::new(),
        };
        let contact_relative = tile.contact_point_relative();
        let height = tile.preferred_height();
        tile.y_gauge = if message.common.parallel {
            YGauge::create_parallel(current_y, contact_relative, height)
        } else {
            YGauge::create_with_contact(current_y, contact_relative, height)
        };
        tile
    }

    fn component(&self) -> &ComponentRoseSelfArrow {
        self.component.get_or_init(|| {
            let common = &self.message.common;
            create_component_self_arrow(
                &message_style(common),
                &common.arrow_configuration.self_arrow(),
                self.arguments.diagram.skin(),
                &MessageLabel {
                    number: common.message_number.as_deref(),
                    display: &common.label,
                },
            )
        })
    }

    pub(super) fn living_space1(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.message.participant1)
    }

    fn is_reverse_define(&self) -> bool {
        self.message.common.arrow_configuration.is_reverse_define()
    }

    fn component_width(&self) -> f64 {
        self.component()
            .preferred_dimension(self.arguments.string_bounder())
            .width
    }

    fn drawn_width(&self) -> f64 {
        self.component()
            .drawn_width(self.arguments.string_bounder())
    }

    fn level(&self, mode: EventsHistoryMode) -> usize {
        self.living_space1().level_at(self.event, mode)
    }

    /// One activation width more on the left when the lifeline is active there.
    fn live_delta_adjustment(&self) -> f64 {
        if self.level(EventsHistoryMode::IgnoreFutureActivate) > 0 {
            LIVE_DELTA_SIZE
        } else {
            0.0
        }
    }

    /// Makes the participant's own box clear the loop, which may stick out past it.
    fn ensure_own_box_clears_loop(&self) {
        let string_bounder = self.arguments.string_bounder();
        let living_space1 = self.living_space1();
        if self.is_reverse_define() {
            let overflow_left =
                living_space1.pos_b().current_value() - self.min_x().current_value();
            if overflow_left > 0.0 {
                living_space1.ensure_margin_before(overflow_left);
            }
        } else {
            let overflow_right =
                self.max_x().current_value() - living_space1.pos_d(string_bounder).current_value();
            if overflow_right > 0.0 {
                living_space1.ensure_margin_after(overflow_right);
            }
        }
    }
}

impl<'a> Tile<'a> for CommunicationTileSelf<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.component().y_point(self.arguments.string_bounder())
    }

    fn preferred_height(&self) -> f64 {
        self.component()
            .preferred_dimension(self.arguments.string_bounder())
            .height
    }

    fn on_gauge_resolved(&self) {
        let string_bounder = self.arguments.string_bounder();
        let component = self.component();
        let dimension = component.preferred_dimension(string_bounder);
        let start = component.start_point(string_bounder, dimension);
        let end = component.end_point(string_bounder, dimension);
        let y = self.y_gauge.min.current_value();
        let common = &self.message.common;
        let step = if common.is_activate() {
            Some(end.y)
        } else if common.is_deactivate() {
            Some(start.y)
        } else if common.is_destroy() {
            Some(end.y)
        } else {
            None
        };
        if let Some(step) = step {
            self.living_space1()
                .add_step_for_livebox(self.event, y + step);
        }
    }

    fn add_constraints(&self) {
        if self.arguments.diagram.sequence_message_span() {
            return;
        }
        let string_bounder = self.arguments.string_bounder();
        let living_spaces = &self.arguments.living_spaces;
        let participant = self.message.participant1;
        if self.is_reverse_define() {
            if let Some(previous) = living_spaces.previous(participant) {
                self.living_space1()
                    .pos_c(string_bounder)
                    .ensure_bigger_than(
                        &previous
                            .pos_c2(string_bounder)
                            .add_fixed(self.component_width()),
                    );
            }
        } else if let Some(next) = living_spaces.next(participant) {
            next.pos_c(string_bounder).ensure_bigger_than(&self.max_x());
        }
        self.ensure_own_box_clears_loop();
    }

    fn min_x(&self) -> Real {
        let pos_c = self.living_space1().pos_c(self.arguments.string_bounder());
        if self.is_reverse_define() {
            return pos_c.add_fixed(-self.component_width() - self.live_delta_adjustment());
        }
        pos_c
    }

    fn max_x(&self) -> Real {
        let pos_c2 = self.living_space1().pos_c2(self.arguments.string_bounder());
        if self.is_reverse_define() {
            return pos_c2;
        }
        pos_c2.add_fixed(self.component_width())
    }

    fn drawn_min_x(&self) -> Real {
        if self.is_reverse_define() {
            return self
                .living_space1()
                .pos_c(self.arguments.string_bounder())
                .add_fixed(-self.drawn_width() - self.live_delta_adjustment());
        }
        self.min_x()
    }

    fn drawn_max_x(&self) -> Real {
        if self.is_reverse_define() {
            return self.max_x();
        }
        self.living_space1()
            .pos_c2(self.arguments.string_bounder())
            .add_fixed(self.drawn_width())
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let string_bounder = ug.string_bounder();
        let component = self.component();
        let dimension = component.preferred_dimension(string_bounder);
        let mut x1 = self.min_x().current_value();
        let level_ignore = self.level(EventsHistoryMode::IgnoreFutureActivate) as f64;
        let level_consider = self.level(EventsHistoryMode::ConsiderFutureDeactivate) as f64;
        if !self.is_reverse_define() {
            x1 += LIVE_DELTA_SIZE * level_ignore;
            if level_ignore < level_consider {
                x1 += LIVE_DELTA_SIZE * (level_consider - level_ignore);
            }
        }
        let area = Area {
            delta_x1: (level_ignore - level_consider) * LIVE_DELTA_SIZE,
            level: level_ignore as i32,
            live_delta_size: LIVE_DELTA_SIZE,
            ..Area::new(dimension.width, dimension.height)
        };
        component.draw_u(&ug.translated(x1, 0.0), &area, context);
    }

    fn stable_min_x(&self) -> Vec<Real> {
        vec![self.drawn_min_x()]
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }
}
