//! Messages from or to the diagram's border (PlantUML's `CommunicationExoTile`).

use std::rc::Rc;

use super::communication::LIVE_DELTA_SIZE;
use super::components;
use super::living_space::{EventsHistoryMode, LivingSpace};
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, MessageExo, MessageExoType};
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::arrow::ArrowDecoration;
use crate::skin::component::{Area, ArrowComponent, Context2D};
use crate::skin::rose::arrow::DIAM_CIRCLE;

pub(super) struct CommunicationExoTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    message: &'a MessageExo,
    y_gauge: YGauge,
    component: Box<dyn ArrowComponent>,
}

impl<'a> CommunicationExoTile<'a> {
    pub(super) fn new(
        arguments: Rc<TileArguments<'a>>,
        event: EventId,
        message: &'a MessageExo,
        current_y: &YGauge,
    ) -> Self {
        let configuration = &message.common.arrow_configuration;
        let configuration = if message.kind.direction() == -1 {
            configuration.reverse()
        } else {
            configuration.clone()
        };
        let component =
            components::message_arrow(arguments.diagram, &message.common, &configuration);
        let mut tile = Self {
            arguments,
            event,
            message,
            y_gauge: current_y.clone(),
            component,
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

    fn living_space(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.message.participant)
    }

    fn preferred_width(&self) -> f64 {
        self.component
            .preferred_dimension(self.arguments.string_bounder())
            .width
    }

    fn point1(&self) -> Real {
        let pos_c = self.living_space().pos_c(self.arguments.string_bounder());
        if self.message.kind.is_right_border() {
            pos_c
        } else {
            pos_c.add_fixed(-self.preferred_width())
        }
    }

    fn point2(&self) -> Real {
        if self.message.kind.is_left_border() {
            self.living_space().pos_c(self.arguments.string_bounder())
        } else {
            self.point1().add_fixed(self.preferred_width())
        }
    }

    fn point1_value(&self) -> f64 {
        if self.is_from_left_border_message() {
            self.arguments.border1()
        } else {
            self.point1().current_value()
        }
    }

    fn point2_value(&self) -> f64 {
        if self.is_from_right_border_message() {
            self.arguments.border2()
        } else {
            self.point2().current_value()
        }
    }

    /// Short arrows (`?->`) stop next to the participant instead of reaching the border.
    fn is_from_right_border_message(&self) -> bool {
        self.message.kind.is_right_border() && !self.message.short_arrow
    }

    fn is_from_left_border_message(&self) -> bool {
        self.message.kind.is_left_border() && !self.message.short_arrow
    }
}

impl<'a> Tile<'a> for CommunicationExoTile<'a> {
    fn event(&self) -> EventId {
        self.event
    }

    fn y_gauge(&self) -> &YGauge {
        &self.y_gauge
    }

    fn contact_point_relative(&self) -> f64 {
        self.component.y_point(self.arguments.string_bounder())
    }

    fn preferred_height(&self) -> f64 {
        self.component
            .preferred_dimension(self.arguments.string_bounder())
            .height
    }

    fn on_gauge_resolved(&self) {
        let string_bounder = self.arguments.string_bounder();
        let dimension = self.component.preferred_dimension(string_bounder);
        let arrow_y = self.component.start_point(string_bounder, dimension).y;
        self.living_space()
            .add_step_for_livebox(self.event, self.y_gauge.min.current_value() + arrow_y);
    }

    fn add_constraints(&self) {
        if !self.message.kind.is_right_border() {
            self.living_space()
                .pos_c(self.arguments.string_bounder())
                .ensure_bigger_than(&self.arguments.x_origin.add_fixed(self.preferred_width()));
        }
    }

    fn min_x(&self) -> Real {
        self.point1()
    }

    fn max_x(&self) -> Real {
        self.point2()
    }

    fn middle_x(&self) -> f64 {
        if !self.is_from_left_border_message() {
            return f64::midpoint(self.min_x().current_value(), self.max_x().current_value());
        }
        let min = self.point1_value();
        let max = min + self.preferred_width();
        f64::midpoint(min, max)
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let string_bounder = ug.string_bounder();
        let dimension = self.component.preferred_dimension(string_bounder);
        let mut x1 = self.point1_value();
        let mut x2 = self.point2_value();
        let text_delta_x = if self.is_from_left_border_message() {
            self.point1().current_value() - x1
        } else if self.is_from_right_border_message() {
            self.point2().current_value() - x2
        } else {
            0.0
        };
        let level =
            self.living_space()
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate) as i64;
        let kind = self.message.kind;
        if level > 0 {
            if kind.is_right_border() {
                x1 += LIVE_DELTA_SIZE * level as f64;
            } else {
                x2 += LIVE_DELTA_SIZE * (level - 2) as f64;
            }
        }
        let configuration = &self.message.common.arrow_configuration;
        let circle1 = configuration.decoration1() == ArrowDecoration::Circle;
        let circle2 = configuration.decoration2() == ArrowDecoration::Circle;
        let circle_room = DIAM_CIRCLE / 2.0 + 2.0;
        if circle1 && kind == MessageExoType::FromLeft {
            x1 += circle_room;
        }
        if circle2 && kind == MessageExoType::ToLeft {
            x1 += circle_room;
        }
        if circle2 && kind == MessageExoType::ToRight {
            x2 -= circle_room;
        }
        if circle1 && kind == MessageExoType::FromRight {
            x2 -= circle_room;
        }
        let area = Area {
            text_delta_x,
            ..Area::new(x2 - x1, dimension.height)
        };
        self.component
            .draw_u(&ug.translated(x1, 0.0), &area, context);
    }
}
