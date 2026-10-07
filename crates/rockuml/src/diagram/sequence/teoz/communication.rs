//! Messages between two participants (PlantUML's `CommunicationTile`).

use std::cell::OnceCell;
use std::rc::Rc;

use super::components;
use super::living_space::{EventsHistoryMode, LivingSpace};
use super::tile::{Tile, TileArguments};
use super::y_gauge::YGauge;
use crate::diagram::sequence::model::{EventId, Message};
use crate::klimt::ugraphic::UGraphic;
use crate::real::Real;
use crate::skin::component::{Area, ArrowComponent, Context2D};

/// How far each activation level shifts an arrow's end.
pub(super) const LIVE_DELTA_SIZE: f64 = 5.0;

pub(super) struct CommunicationTile<'a> {
    arguments: Rc<TileArguments<'a>>,
    event: EventId,
    message: &'a Message,
    y_gauge: YGauge,
    /// The arrow, drawn left to right or reversed.
    components: [OnceCell<Box<dyn ArrowComponent>>; 2],
}

impl<'a> CommunicationTile<'a> {
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
            components: [OnceCell::new(), OnceCell::new()],
        };
        if message.common.is_create() {
            tile.living_space2().go_create();
        }
        let string_bounder = tile.arguments.string_bounder.clone();
        let contact_relative = tile.component().y_point(string_bounder.as_ref());
        let height = tile.preferred_height();
        tile.y_gauge = if message.common.parallel {
            YGauge::create_parallel(current_y, contact_relative, height)
        } else {
            YGauge::create_with_contact(current_y, contact_relative, height)
        };
        tile
    }

    fn living_space1(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.message.participant1)
    }

    fn living_space2(&self) -> &LivingSpace<'a> {
        self.arguments.living_space(self.message.participant2)
    }

    /// Whether the message goes right to left on the page.
    pub(super) fn is_reverse(&self) -> bool {
        let string_bounder = self.arguments.string_bounder();
        self.living_space1().pos_c(string_bounder).current_value()
            > self.living_space2().pos_c(string_bounder).current_value()
    }

    fn component(&self) -> &dyn ArrowComponent {
        let reverse = self.is_reverse();
        self.components[usize::from(reverse)]
            .get_or_init(|| {
                let configuration = &self.message.common.arrow_configuration;
                let configuration = if reverse {
                    configuration.reverse()
                } else {
                    configuration.clone()
                };
                components::message_arrow(
                    self.arguments.diagram,
                    &self.message.common,
                    &configuration,
                )
            })
            .as_ref()
    }

    fn point1(&self) -> Real {
        self.living_space1().pos_c(self.arguments.string_bounder())
    }

    /// Creation messages end at the created participant's head.
    fn point2(&self) -> Real {
        let string_bounder = self.arguments.string_bounder();
        let living_space2 = self.living_space2();
        if self.message.common.is_create() {
            if self.is_reverse() {
                living_space2.pos_d(string_bounder)
            } else {
                living_space2.pos_b().clone()
            }
        } else {
            living_space2.pos_c(string_bounder)
        }
    }

    fn levels(&self) -> (i64, i64) {
        let mode = EventsHistoryMode::IgnoreFutureDeactivate;
        (
            self.living_space1().level_at(self.event, mode) as i64,
            self.living_space2().level_at(self.event, mode) as i64,
        )
    }

    fn arrow_width(&self) -> f64 {
        self.component()
            .preferred_dimension(self.arguments.string_bounder())
            .width
    }
}

impl<'a> Tile<'a> for CommunicationTile<'a> {
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
        let string_bounder = self.arguments.string_bounder();
        let height = self.component().preferred_dimension(string_bounder).height;
        if self.message.common.is_create() {
            return height.max(
                self.living_space2()
                    .head_preferred_dimension(string_bounder)
                    .height,
            );
        }
        height
    }

    fn on_gauge_resolved(&self) {
        let y = self.y_gauge.min.current_value();
        if self.message.common.is_create() {
            self.living_space2().go_create_at(y);
        }
        let string_bounder = self.arguments.string_bounder();
        let component = self.component();
        let dimension = component.preferred_dimension(string_bounder);
        let arrow_y = component.start_point(string_bounder, dimension).y;
        self.living_space1()
            .add_step_for_livebox(self.event, y + arrow_y);
        self.living_space2()
            .add_step_for_livebox(self.event, y + arrow_y);
    }

    fn add_constraints(&self) {
        if self.arguments.diagram.sequence_message_span() {
            return;
        }
        let width = self.arrow_width();
        let mut point1 = self.point1();
        let mut point2 = self.point2();
        let (level1, level2) = self.levels();
        if self.is_reverse() {
            if level1 > 0 {
                point1 = point1.add_fixed(-LIVE_DELTA_SIZE);
            }
            point2 = point2.add_fixed(level2 as f64 * LIVE_DELTA_SIZE);
            point1.ensure_bigger_than(&point2.add_fixed(width));
        } else {
            if level2 > 0 {
                point2 = point2.add_fixed(-LIVE_DELTA_SIZE);
            }
            point2.ensure_bigger_than(&point1.add_fixed(width));
        }
    }

    fn min_x(&self) -> Real {
        if self.is_reverse() {
            self.point2()
        } else {
            self.point1()
        }
    }

    fn max_x(&self) -> Real {
        let width = self.arrow_width();
        let (left, right) = if self.is_reverse() {
            (self.point2(), self.point1())
        } else {
            (self.point1(), self.point2())
        };
        Real::max(vec![right, left.add_fixed(width)])
    }

    fn draw_u(&self, ug: &UGraphic, context: Context2D) {
        let ug = ug.translated(0.0, self.y_gauge.min.current_value());
        let common = &self.message.common;
        if common.part1_anchor.is_some() || common.part2_anchor.is_some() {
            return;
        }
        let string_bounder = ug.string_bounder();
        let component = self.component();
        let dimension = component.preferred_dimension(string_bounder);
        let mut x1 = self.point1().current_value();
        let mut x2 = self.point2().current_value();
        let (level1, mut level2) = self.levels();
        let (area, ug) = if self.is_reverse() {
            if level1 == 1 {
                x1 -= LIVE_DELTA_SIZE;
            } else if level1 > 2 {
                x1 += LIVE_DELTA_SIZE * (level1 - 2) as f64;
            }
            x2 += LIVE_DELTA_SIZE * level2 as f64;
            let ug = ug.translated(x2, 0.0);
            if common.is_create() {
                self.living_space2().draw_created_head(&ug, context, true);
            }
            (Area::new(x1 - x2, dimension.height), ug)
        } else {
            if level2 > 0 {
                level2 -= 2;
            }
            x1 += LIVE_DELTA_SIZE * level1 as f64;
            x2 += LIVE_DELTA_SIZE * level2 as f64;
            let area = Area::new(x2 - x1, dimension.height);
            let ug = ug.translated(x1, 0.0);
            if common.is_create() {
                self.living_space2().draw_created_head(
                    &ug.translated(area.dimension.width, 0.0),
                    context,
                    false,
                );
            }
            (area, ug)
        };
        component.draw_u(&ug, &area, context);
    }
}
