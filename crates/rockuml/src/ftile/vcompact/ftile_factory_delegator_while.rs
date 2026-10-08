//! PlantUML's `FtileFactoryDelegatorWhile`: builds `while` loops, and joins the `break`s inside to the
//! loop's way out.

use std::rc::Rc;

use super::ftile_while::{FtileWhile, WhileStyle};
use crate::color::HColor;
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::diagram::activity3::{InstructionId, Instructions, LinkRendering, SwimlaneId};
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
// Only the delegator's methods are in scope: the factory's are the same ones, through it.
use crate::ftile::{self, Connection, Ftile, FtileFactoryDelegator, Genealogy, Snake, ftile_utils};
use crate::klimt::ugraphic::UGraphic;
use crate::style::{PName, StyleBuilder, ValueReading};

pub(crate) struct FtileFactoryDelegatorWhile {
    factory: Box<dyn ftile::FtileFactory>,
}

impl FtileFactoryDelegatorWhile {
    pub(crate) fn new(factory: Box<dyn ftile::FtileFactory>) -> Self {
        Self { factory }
    }
}

/// The colours of an arrow, or `color` when it has none (`ensureColor`).
fn ensure_color(link: &LinkRendering, color: &Rainbow) -> LinkRendering {
    if link.rainbow.size() == 0 {
        return link.with_rainbow(color.clone());
    }
    link.clone()
}

impl FtileFactoryDelegator for FtileFactoryDelegatorWhile {
    fn get_factory(&self) -> &dyn ftile::FtileFactory {
        self.factory.as_ref()
    }

    /// The loop is built with the chain inside this delegator, as PlantUML hands `FtileWhile` the
    /// factory it wraps; so is the `stop` or `end` after it.
    fn create_while(
        &self,
        instructions: &Instructions,
        out_color: &LinkRendering,
        swimlane: Option<SwimlaneId>,
        while_block: Rc<dyn Ftile>,
        test: &Display,
        yes: Option<&Display>,
        color: Option<HColor>,
        special_out: Option<InstructionId>,
        backward: Option<Rc<dyn Ftile>>,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
        current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        let skin_param = self.skin_param();
        let style_arrow = self
            .get_default_style_definition_arrow()
            .get_merged_style(current_style_builder);
        let style_diamond = self
            .get_default_style_definition_diamond()
            .get_merged_style(current_style_builder);
        let arrow_color = Rainbow::build_from_style(&style_arrow);
        let style = WhileStyle {
            border_color: style_diamond.value(PName::LineColor).as_color(),
            back_color: color
                .unwrap_or_else(|| style_diamond.value(PName::BackGroundColor).as_color()),
            arrow_color: arrow_color.clone(),
            font_arrow: style_arrow.font_configuration(),
            condition_style: skin_param.get_condition_style(),
            fc_test: style_diamond.font_configuration(),
        };

        let factory = self.get_factory();
        let mut result = FtileWhile::create(
            &ensure_color(out_color, &arrow_color),
            swimlane,
            while_block.clone(),
            test,
            yes,
            &style,
            factory,
            special_out.map(|special_out| instructions.create_ftile(special_out, factory)),
            backward,
            &ensure_color(incoming1, &arrow_color),
            &ensure_color(incoming2, &arrow_color),
        );

        let welding_points = while_block.get_welding_points();
        if !welding_points.is_empty() {
            let genealogy = Rc::new(Genealogy::new(&result));
            for ftile_break in welding_points {
                result = ftile_utils::add_connection(
                    result,
                    Rc::new(ConnectionBreak {
                        ftile_break,
                        genealogy: genealogy.clone(),
                        arrow_color: arrow_color.clone(),
                    }),
                );
            }
        }
        result
    }
}

/// From a `break` left to the loop's way out.
struct ConnectionBreak {
    ftile_break: Rc<dyn Ftile>,
    genealogy: Rc<Genealogy>,
    arrow_color: Rainbow,
}

impl Connection for ConnectionBreak {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        Some(&self.ftile_break)
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        None
    }

    fn draw_u(&self, ug: &UGraphic) {
        let tr1 = self
            .genealogy
            .get_translate(self.ftile_break.as_ref(), ug.string_bounder());
        let skin_param = self.ftile_break.skin_param();
        let mut snake = Snake::create_with_end(
            skin_param,
            self.arrow_color.clone(),
            skin_param.arrows().as_to_left(),
        );
        snake.add_point(tr1.dx, tr1.dy);
        snake.add_point(HEXAGON_HALF_SIZE, tr1.dy);
        ug.draw(&snake);
    }
}
