//! The arrows of a `switch` with several cases (PlantUML's `FtileSwitchWithManyLinks`).

use std::rc::Rc;

use super::ftile_switch_with_diamonds::{FtileSwitchWithDiamonds, SwitchLinks};
use crate::decoration::Rainbow;
use crate::diagram::activity3::SwimlaneId;
use crate::direction::Direction;
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::{
    AbstractConnection, Connection, ConnectionTranslatable, Ftile, Snake, Swimable, ftile_utils,
    same,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{TextBlock, VerticalAlignment};
use crate::skin::SkinParam;

const MARGIN: f64 = 10.0;

/// The `switch` of `tiles`, its cases, with their arrows (`new FtileSwitchWithManyLinks(...).addLinks()`).
#[allow(clippy::too_many_arguments, reason = "PlantUML's constructor")]
pub(crate) fn create(
    skin_param: Rc<SkinParam>,
    tiles: Vec<Rc<dyn Ftile>>,
    text_block_positives: Vec<Rc<dyn TextBlock>>,
    text_block_specials: Vec<Rc<dyn TextBlock>>,
    in_: Option<SwimlaneId>,
    diamond1: Rc<dyn Ftile>,
    diamond2: Rc<dyn Ftile>,
    string_bounder: &dyn StringBounder,
    arrow_color: Rainbow,
) -> Rc<dyn Ftile> {
    let result = Rc::new(FtileSwitchWithDiamonds::new(
        skin_param,
        tiles,
        text_block_positives,
        text_block_specials,
        in_,
        diamond1,
        diamond2,
        string_bounder,
        arrow_color,
        SwitchLinks::ManyLinks,
    ));
    let mut conns = Vec::new();
    add_ingoing_arrows(&result, &mut conns);
    add_outgoing_arrows(&result, string_bounder, &mut conns);
    ftile_utils::add_connections(result, conns)
}

fn connection(
    parent: &Rc<FtileSwitchWithDiamonds>,
    kind: ConnectionKind,
    ftile1: &Rc<dyn Ftile>,
    ftile2: &Rc<dyn Ftile>,
    label: Option<Rc<dyn TextBlock>>,
) -> Rc<dyn Connection> {
    Rc::new(ConnectionSwitch {
        parent: Rc::clone(parent),
        base: AbstractConnection::new(Some(Rc::clone(ftile1)), Some(Rc::clone(ftile2))),
        kind,
        label,
    })
}

fn add_ingoing_arrows(parent: &Rc<FtileSwitchWithDiamonds>, conns: &mut Vec<Rc<dyn Connection>>) {
    let tiles = &parent.tiles;
    let labels = &parent.text_block_positives;
    let (Some(first), Some(last)) = (tiles.first(), tiles.last()) else {
        return;
    };
    let diamond1 = &parent.diamond1;
    conns.push(connection(
        parent,
        ConnectionKind::HorizontalThenVertical,
        diamond1,
        first,
        labels.first().cloned(),
    ));
    conns.push(connection(
        parent,
        ConnectionKind::HorizontalThenVertical,
        diamond1,
        last,
        labels.last().cloned(),
    ));
    for (i, (tile, label)) in tiles.iter().zip(labels).enumerate() {
        if i > 0 && i < tiles.len() - 1 {
            conns.push(connection(
                parent,
                ConnectionKind::VerticalTop,
                diamond1,
                tile,
                Some(Rc::clone(label)),
            ));
        }
        if different_swimlane(Some(parent.as_ref()), Some(tile.as_ref())) {
            conns.push(connection(
                parent,
                ConnectionKind::HorizontalThenVerticalCrossSwimlane,
                diamond1,
                tile,
                Some(Rc::clone(label)),
            ));
        }
    }
}

fn add_outgoing_arrows(
    parent: &Rc<FtileSwitchWithDiamonds>,
    string_bounder: &dyn StringBounder,
    conns: &mut Vec<Rc<dyn Connection>>,
) {
    let tiles = &parent.tiles;
    let specials = &parent.text_block_specials;
    let diamond2 = &parent.diamond2;
    let goes_on_here = |tile: &Rc<dyn Ftile>| {
        tile.calculate_dimension(string_bounder).has_point_out()
            && !different_swimlane(Some(parent.as_ref()), Some(tile.as_ref()))
    };
    let Some(last_outgoing_arrow) = tiles.iter().rposition(goes_on_here) else {
        return;
    };
    let first_outgoing_arrow = tiles[..tiles.len() - 1]
        .iter()
        .position(goes_on_here)
        .unwrap_or(tiles.len());
    if first_outgoing_arrow < tiles.len() {
        conns.push(connection(
            parent,
            ConnectionKind::VerticalThenHorizontal,
            &tiles[first_outgoing_arrow],
            diamond2,
            Some(Rc::clone(&specials[first_outgoing_arrow])),
        ));
    }
    if last_outgoing_arrow > 0 {
        conns.push(connection(
            parent,
            ConnectionKind::VerticalThenHorizontal,
            &tiles[last_outgoing_arrow],
            diamond2,
            Some(Rc::clone(&specials[last_outgoing_arrow])),
        ));
    }
    for i in first_outgoing_arrow + 1..last_outgoing_arrow {
        let tile = &tiles[i];
        if tile.calculate_dimension(string_bounder).has_point_out() {
            conns.push(connection(
                parent,
                ConnectionKind::VerticalBottom,
                tile,
                diamond2,
                Some(Rc::clone(&specials[i])),
            ));
        }
    }
    for tile in tiles {
        if let Some(origin) = find_last_with_point_out(tile, string_bounder)
            && different_swimlane(Some(origin.as_ref()), Some(diamond2.as_ref()))
        {
            conns.push(connection(
                parent,
                ConnectionKind::VerticalThenHorizontalCrossSwimlane,
                &origin,
                diamond2,
                None,
            ));
        }
    }
}

/// `tile`, or else the first of its descendants, depth first, that the flow leaves.
fn find_last_with_point_out(
    tile: &Rc<dyn Ftile>,
    string_bounder: &dyn StringBounder,
) -> Option<Rc<dyn Ftile>> {
    if tile.calculate_dimension(string_bounder).has_point_out() {
        return Some(Rc::clone(tile));
    }
    tile.get_my_children()
        .iter()
        .find_map(|child| find_last_with_point_out(child, string_bounder))
}

/// Whether the flow leaves `ftile1` and enters `ftile2` in lanes known to differ (`differentSwimlane`).
fn different_swimlane(ftile1: Option<&dyn Swimable>, ftile2: Option<&dyn Swimable>) -> bool {
    let swimlane1 = ftile1.and_then(Swimable::get_swimlane_out);
    let swimlane2 = ftile2.and_then(Swimable::get_swimlane_in);
    matches!((swimlane1, swimlane2), (Some(lane1), Some(lane2)) if lane1 != lane2)
}

/// PlantUML's inner connection classes of `FtileSwitchWithManyLinks`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConnectionKind {
    /// From a side of the first diamond into the first or the last case.
    HorizontalThenVertical,
    /// From a case to a side or the bottom of the second diamond.
    VerticalThenHorizontal,
    /// From the first diamond into a case between the first and the last.
    VerticalTop,
    /// From a case between the first and the last to the second diamond.
    VerticalBottom,
    /// Into a case in another lane; drawn only across lanes.
    HorizontalThenVerticalCrossSwimlane,
    /// Out of a case in another lane; drawn only across lanes.
    VerticalThenHorizontalCrossSwimlane,
}

struct ConnectionSwitch {
    parent: Rc<FtileSwitchWithDiamonds>,
    base: AbstractConnection,
    kind: ConnectionKind,
    label: Option<Rc<dyn TextBlock>>,
}

impl ConnectionSwitch {
    fn ftile1(&self) -> &Rc<dyn Ftile> {
        self.base
            .get_ftile1()
            .expect("switch connections join two tiles")
    }

    fn ftile2(&self) -> &Rc<dyn Ftile> {
        self.base
            .get_ftile2()
            .expect("switch connections join two tiles")
    }

    fn skin_param(&self) -> &SkinParam {
        self.parent.skin_param()
    }

    fn snake_down(&self) -> Snake {
        Snake::create_with_end(
            self.skin_param(),
            self.parent.arrow_color.clone(),
            self.skin_param().arrows().as_to_down(),
        )
    }

    /// The point out of `ftile1`, a case.
    fn case_out(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = self.ftile1();
        self.parent
            .get_translate_of(tile.as_ref(), string_bounder)
            .get_translated(tile.calculate_dimension(string_bounder).get_point_out())
    }

    /// The point in of `ftile2`, a case.
    fn case_in(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = self.ftile2();
        self.parent
            .get_translate_of(tile.as_ref(), string_bounder)
            .get_translated(tile.calculate_dimension(string_bounder).get_point_in())
    }

    fn is_last(&self) -> bool {
        self.parent
            .tiles
            .last()
            .is_some_and(|last| same(self.ftile2().as_ref(), last.as_ref()))
    }

    fn draw_horizontal_then_vertical(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let is_first = parent
            .tiles
            .first()
            .is_some_and(|first| same(self.ftile2().as_ref(), first.as_ref()));
        let pt = if is_first {
            dim_diamond1.get_point_d()
        } else {
            dim_diamond1.get_point_b()
        };
        let p1 = parent
            .get_translate_diamond1(string_bounder)
            .get_translated(pt);
        let p2 = self.case_in(string_bounder);
        let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
        let mut snake = self
            .snake_down()
            .with_label(self.label.clone(), self.base.arrow_horizontal_alignment());
        snake.add_point(x1, y1);
        if self.is_last() && p1.x > p2.x {
            snake.add_point(x1 + HEXAGON_HALF_SIZE, y1);
            snake.add_point(x1 + HEXAGON_HALF_SIZE, y1 + dim_diamond1.get_height());
            snake.add_point(x2, y1 + dim_diamond1.get_height());
        } else {
            snake.add_point(x2, y1);
        }
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }

    fn draw_vertical_then_horizontal(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        if !self
            .ftile1()
            .calculate_dimension(string_bounder)
            .has_point_out()
        {
            return;
        }
        let parent = &self.parent;
        let p1 = self.case_out(string_bounder);
        let (x1, y1) = (p1.x, p1.y);
        let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
        let translate_diamond2 = parent.get_translate_diamond2(string_bounder);
        let pt_a = translate_diamond2.get_translated(dim_diamond2.get_point_a());
        let pt_b = translate_diamond2.get_translated(dim_diamond2.get_point_b());
        let pt_d = translate_diamond2.get_translated(dim_diamond2.get_point_d());
        let arrows = self.skin_param().arrows();
        let (p2, arrow, direction) = if x1 < pt_d.x {
            (pt_d, arrows.as_to_right(), Direction::Right)
        } else if x1 > pt_b.x {
            (pt_b, arrows.as_to_left(), Direction::Left)
        } else {
            (pt_a, arrows.as_to_down(), Direction::Down)
        };
        let (x2, y2) = (p2.x, p2.y);
        let mut snake =
            Snake::create_with_end(self.skin_param(), parent.arrow_color.clone(), arrow)
                .with_label_vertical(self.label.clone(), VerticalAlignment::Center);
        snake.add_point(x1, y1);
        if direction == Direction::Left && x2 > x1 - 10.0 {
            snake.add_point(x1, y2 - 8.0);
            snake.add_point(x1 + 12.0, y2 - 8.0);
            snake.add_point(x1 + 12.0, y2);
        } else {
            snake.add_point(x1, y2);
        }
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }

    /// The first diamond's points `b`, `c` and `d`, where it is drawn.
    fn diamond1_points(
        &self,
        string_bounder: &dyn StringBounder,
    ) -> (XPoint2D, XPoint2D, XPoint2D) {
        let parent = &self.parent;
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let translate_diamond1 = parent.get_translate_diamond1(string_bounder);
        (
            translate_diamond1.get_translated(dim_diamond1.get_point_b()),
            translate_diamond1.get_translated(dim_diamond1.get_point_c()),
            translate_diamond1.get_translated(dim_diamond1.get_point_d()),
        )
    }

    fn draw_vertical_top(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let (p1b, p1c, p1d) = self.diamond1_points(string_bounder);
        let p2 = self.case_in(string_bounder);
        let (x2, y2) = (p2.x, p2.y);
        let mut snake = self
            .snake_down()
            .with_label_vertical(self.label.clone(), VerticalAlignment::Center);
        if x2 < p1d.x - MARGIN || x2 > p1b.x + MARGIN {
            snake.add_point(x2, p1d.y);
            snake.add_point(x2, y2);
        } else {
            let (x1, y1) = (p1c.x, p1c.y);
            let ym = (y1 * 2.0 + y2) / 3.0;
            snake.add_point(x1, y1);
            snake.add_point(x1, ym);
            snake.add_point(x2, ym);
            snake.add_point(x2, y2);
        }
        ug.draw(&snake);
    }

    fn draw_vertical_bottom(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let p1 = self.case_out(string_bounder);
        let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
        let translate_diamond2 = parent.get_translate_diamond2(string_bounder);
        let p2a = translate_diamond2.get_translated(dim_diamond2.get_point_a());
        let p2d = translate_diamond2.get_translated(dim_diamond2.get_point_d());
        let (p1b, _, p1d) = self.diamond1_points(string_bounder);
        let (x1, y1) = (p1.x, p1.y);
        let (x2, y2) = (p2a.x, p2a.y);
        let ym = f64::midpoint(y1, y2);
        let mut snake = self
            .snake_down()
            .with_label_vertical(self.label.clone(), VerticalAlignment::Center);
        if x1 < p1d.x - MARGIN || x1 > p1b.x + MARGIN {
            snake.add_point(x1, y1);
            snake.add_point(x1, p2d.y);
        } else {
            snake.add_point(x1, y1);
            snake.add_point(x1, ym);
            snake.add_point(x2, ym);
            snake.add_point(x2, y2);
        }
        ug.draw(&snake);
    }

    /// The first diamond's point out, where it is drawn, and the point in of the case.
    fn cross_in_points(&self, string_bounder: &dyn StringBounder) -> (XPoint2D, XPoint2D) {
        let p1 = self
            .parent
            .get_translate_diamond1(string_bounder)
            .get_translated(
                self.ftile1()
                    .calculate_dimension(string_bounder)
                    .get_point_out(),
            );
        (p1, self.case_in(string_bounder))
    }
}

impl Connection for ConnectionSwitch {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        match self.kind {
            ConnectionKind::HorizontalThenVertical => self.draw_horizontal_then_vertical(ug),
            ConnectionKind::VerticalThenHorizontal => self.draw_vertical_then_horizontal(ug),
            ConnectionKind::VerticalTop => self.draw_vertical_top(ug),
            ConnectionKind::VerticalBottom => self.draw_vertical_bottom(ug),
            ConnectionKind::HorizontalThenVerticalCrossSwimlane => {
                let (p1, p2) = self.cross_in_points(ug.string_bounder());
                let mut snake = self.snake_down();
                snake.add_point_at(p1);
                snake.add_point_at(p2);
                ug.draw(&snake);
            }
            ConnectionKind::VerticalThenHorizontalCrossSwimlane => {}
        }
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        match self.kind {
            ConnectionKind::HorizontalThenVerticalCrossSwimlane
            | ConnectionKind::VerticalThenHorizontalCrossSwimlane => Some(self),
            _ => None,
        }
    }
}

impl ConnectionTranslatable for ConnectionSwitch {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        match self.kind {
            ConnectionKind::HorizontalThenVerticalCrossSwimlane => {
                let (p1, p2) = self.cross_in_points(string_bounder);
                let from = translate1.get_translated(p1);
                let to = translate2.get_translated(p2);
                let mut snake = self
                    .snake_down()
                    .with_label(self.label.clone(), self.base.arrow_horizontal_alignment());
                let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
                let y = p1.y - dim_diamond1.get_height() / 2.0;
                if from.x > to.x {
                    snake.add_point(from.x - dim_diamond1.get_width() / 2.0, y);
                } else {
                    snake.add_point(from.x + dim_diamond1.get_width() / 2.0, y);
                }
                snake.add_point(to.x, y);
                snake.add_point_at(to);
                ug.draw(&snake);
            }
            ConnectionKind::VerticalThenHorizontalCrossSwimlane => {
                let from = translate1.get_translated(self.case_out(string_bounder));
                let p2 = parent
                    .get_translate_diamond2(string_bounder)
                    .get_translated(
                        self.ftile2()
                            .calculate_dimension(string_bounder)
                            .get_point_in(),
                    );
                let to = translate2.get_translated(p2);
                let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
                let arrows = self.skin_param().arrows();
                let left = from.x > to.x;
                let arrow = if left {
                    arrows.as_to_left()
                } else {
                    arrows.as_to_right()
                };
                let mut snake =
                    Snake::create_with_end(self.skin_param(), parent.arrow_color.clone(), arrow);
                let y = to.y + dim_diamond2.get_height() / 2.0;
                snake.add_point_at(from);
                snake.add_point(from.x, y);
                if left {
                    snake.add_point(to.x + dim_diamond2.get_width() / 2.0, y);
                } else {
                    snake.add_point(to.x - dim_diamond2.get_width() / 2.0, y);
                }
                ug.draw(&snake);
            }
            _ => {}
        }
    }
}
