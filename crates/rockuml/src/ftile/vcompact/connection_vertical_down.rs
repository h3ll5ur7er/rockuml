//! The arrow straight down from one tile to the next (PlantUML's `ConnectionVerticalDown`).

use std::rc::Rc;

use crate::decoration::Rainbow;
use crate::ftile::{AbstractConnection, Connection, ConnectionTranslatable, Ftile, Snake};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;

pub(crate) struct ConnectionVerticalDown {
    base: AbstractConnection,
    p1: XPoint2D,
    p2: XPoint2D,
    color: Rainbow,
    text_block: Option<Rc<dyn TextBlock>>,
}

impl ConnectionVerticalDown {
    /// From `p1` to `p2`, in `color`, which has colours; `text_block` labels it.
    pub(crate) fn new(
        ftile1: Rc<dyn Ftile>,
        ftile2: Rc<dyn Ftile>,
        p1: XPoint2D,
        p2: XPoint2D,
        color: Rainbow,
        text_block: Option<Rc<dyn TextBlock>>,
    ) -> Self {
        Self {
            base: AbstractConnection::new(Some(ftile1), Some(ftile2)),
            p1,
            p2,
            color,
            text_block,
        }
    }

    /// How far right the arrow and its label reach.
    pub(crate) fn get_max_x(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.get_simple_snake().get_max_x(string_bounder)
    }

    fn create_snake(&self) -> Snake {
        let skin_param = self.ftile1().skin_param();
        Snake::create_with_end(
            skin_param,
            self.color.clone(),
            skin_param.arrows().as_to_down(),
        )
        .with_label(
            self.text_block.clone(),
            self.base.arrow_horizontal_alignment(),
        )
    }

    fn get_simple_snake(&self) -> Snake {
        let mut snake = self.create_snake();
        snake.add_point_at(self.p1);
        snake.add_point_at(self.p2);
        snake
    }

    fn ftile1(&self) -> &Rc<dyn Ftile> {
        self.base.get_ftile1().expect("the arrow leaves a tile")
    }
}

impl Connection for ConnectionVerticalDown {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&self.get_simple_snake());
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionVerticalDown {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let mut snake = self.create_snake();
        let point1 = translate1.get_translated(self.p1);
        let point2 = translate2.get_translated(self.p2);
        let middle = f64::midpoint(point1.y, point2.y);
        snake.add_point_at(point1);
        snake.add_point(point1.x, middle);
        snake.add_point(point2.x, middle);
        snake.add_point_at(point2);
        ug.draw(&snake);
    }
}
