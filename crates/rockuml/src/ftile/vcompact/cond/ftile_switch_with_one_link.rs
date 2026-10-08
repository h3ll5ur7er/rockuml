//! The arrows of a `switch` with a single case (PlantUML's `FtileSwitchWithOneLink`).

use std::rc::Rc;

use super::ftile_switch_with_diamonds::{FtileSwitchWithDiamonds, SwitchLinks};
use crate::decoration::Rainbow;
use crate::diagram::activity3::SwimlaneId;
use crate::ftile::{AbstractConnection, Connection, Ftile, Snake, ftile_utils};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

/// The `switch` of `tile`, its one case, with its arrows (`new FtileSwitchWithOneLink(...).addLinks()`).
#[allow(clippy::too_many_arguments, reason = "PlantUML's constructor")]
pub(crate) fn create(
    skin_param: Rc<SkinParam>,
    tile: Rc<dyn Ftile>,
    text_block_positive: Rc<dyn TextBlock>,
    text_block_special: Rc<dyn TextBlock>,
    in_: Option<SwimlaneId>,
    diamond1: Rc<dyn Ftile>,
    diamond2: Rc<dyn Ftile>,
    string_bounder: &dyn StringBounder,
    arrow_color: Rainbow,
) -> Rc<dyn Ftile> {
    let result = Rc::new(FtileSwitchWithDiamonds::new(
        skin_param,
        vec![Rc::clone(&tile)],
        vec![text_block_positive],
        vec![text_block_special],
        in_,
        diamond1,
        diamond2,
        string_bounder,
        arrow_color,
        SwitchLinks::OneLink,
    ));
    let mut conns: Vec<Rc<dyn Connection>> = vec![Rc::new(ConnectionVerticalTop {
        base: AbstractConnection::new(Some(Rc::clone(&result.diamond1)), Some(Rc::clone(&tile))),
        parent: Rc::clone(&result),
    })];
    if tile.calculate_dimension(string_bounder).has_point_out() {
        conns.push(Rc::new(ConnectionVerticalBottom {
            base: AbstractConnection::new(Some(tile), Some(Rc::clone(&result.diamond2))),
            parent: Rc::clone(&result),
        }));
    }
    ftile_utils::add_connections(result, conns)
}

/// From the bottom of the first diamond down into the case.
struct ConnectionVerticalTop {
    parent: Rc<FtileSwitchWithDiamonds>,
    base: AbstractConnection,
}

impl Connection for ConnectionVerticalTop {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let Some(tile) = self.base.get_ftile2() else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let p1 = parent
            .get_translate_diamond1(string_bounder)
            .get_translated(dim_diamond1.get_point_c());
        let p2 = parent
            .get_translate_of(tile.as_ref(), string_bounder)
            .get_translated(tile.calculate_dimension(string_bounder).get_point_in());
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(
            skin_param,
            parent.arrow_color.clone(),
            skin_param.arrows().as_to_down(),
        )
        .with_label(
            parent.text_block_positives.first().cloned(),
            self.base.arrow_horizontal_alignment(),
        );
        snake.add_point(p2.x, p1.y);
        snake.add_point(p2.x, p2.y);
        ug.draw(&snake);
    }
}

/// From the case down into the second diamond.
struct ConnectionVerticalBottom {
    parent: Rc<FtileSwitchWithDiamonds>,
    base: AbstractConnection,
}

impl Connection for ConnectionVerticalBottom {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let Some(tile) = self.base.get_ftile1() else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let p1 = parent
            .get_translate_of(tile.as_ref(), string_bounder)
            .get_translated(tile.calculate_dimension(string_bounder).get_point_out());
        let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
        let p2 = parent
            .get_translate_diamond2(string_bounder)
            .get_translated(dim_diamond2.get_point_a());
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(
            skin_param,
            parent.arrow_color.clone(),
            skin_param.arrows().as_to_down(),
        );
        snake.add_point(p2.x, p1.y);
        snake.add_point(p2.x, p2.y);
        ug.draw(&snake);
    }
}
