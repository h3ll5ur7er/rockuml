//! An `if` with one branch below the diamond and the other going round it, or ending at once in a stop
//! beside it (PlantUML's `FtileIfDown`).
//!
//! A note on the diamond (the opale) is not ported: it takes no room.

use std::rc::Rc;

use crate::decoration::Rainbow;
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::direction::Direction;
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::vertical::{FtileDiamond, FtileDiamondInside};
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, ConnectionTranslatable, Ftile, FtileEmpty,
    FtileFactory, FtileGeometry, MergeStrategy, Snake, Swimable, downcast, ftile_utils, same,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::shape::{UPolygon, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::svek::ConditionEndStyle;

pub(crate) struct FtileIfDown {
    base: AbstractFtile,
    then_block: Rc<dyn Ftile>,
    diamond1: Rc<dyn Ftile>,
    diamond2: Rc<dyn Ftile>,
    optional_stop: Option<Rc<dyn Ftile>>,
    condition_end_style: ConditionEndStyle,
}

impl FtileIfDown {
    /// The `if`, `then_block` below `diamond1`. With `optional_stop`, the other branch is that stop beside
    /// the diamond; without, it goes round `then_block` to `diamond2` in `else_color`.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's create")]
    pub(crate) fn create(
        diamond1: Rc<dyn Ftile>,
        diamond2: Rc<dyn Ftile>,
        swimlane: Option<SwimlaneId>,
        then_block: Rc<dyn Ftile>,
        arrow_color: &Rainbow,
        condition_end_style: ConditionEndStyle,
        ftile_factory: &dyn FtileFactory,
        optional_stop: Option<Rc<dyn Ftile>>,
        else_color: &Rainbow,
    ) -> Rc<dyn Ftile> {
        let else_color = else_color.with_default(arrow_color);
        let diamond2 = if optional_stop.is_some() {
            Rc::new(FtileEmpty::new(Rc::clone(ftile_factory.skin_param()), None))
        } else {
            diamond2
        };
        let has_stop = optional_stop.is_some();
        let result = Rc::new(Self {
            base: AbstractFtile::new(Rc::clone(ftile_factory.skin_param())),
            then_block: Rc::clone(&then_block),
            diamond1: Rc::clone(&diamond1),
            diamond2,
            optional_stop,
            condition_end_style,
        });
        let in_color = then_block.get_in_link_rendering().get_rainbow_or(arrow_color);
        let mut conns: Vec<Rc<dyn Connection>> = vec![Rc::new(ConnectionIn {
            base: result.between(&result.diamond1, &result.then_block),
            parent: Rc::clone(&result),
            arrow_color: in_color,
        })];
        let has_point_out1 = then_block
            .calculate_dimension(ftile_factory.get_string_bounder())
            .has_point_out();
        if has_stop {
            conns.push(Rc::new(ConnectionHorizontal {
                base: AbstractConnection::new(
                    Some(Rc::clone(&result.diamond1)),
                    result.optional_stop.clone(),
                ),
                parent: Rc::clone(&result),
                color: else_color,
            }));
        } else if !has_point_out1 {
            conns.push(result.connection_else(else_color, ElseKind::NoDiamond));
        } else if condition_end_style == ConditionEndStyle::Diamond {
            let left = swimlane.is_some_and(|swimlane| {
                swimlane.is_smaller_than_all_others(&then_block.get_swimlanes())
            });
            if left {
                conns.push(result.connection_else(else_color, ElseKind::Else1));
                if let Some(diamond1) = downcast::<FtileDiamondInside>(diamond1.as_ref()) {
                    diamond1.swap_east_west();
                }
            } else {
                conns.push(result.connection_else(else_color, ElseKind::Else2));
            }
        } else {
            conns.push(result.connection_else(else_color.clone(), ElseKind::Hline));
            conns.push(Rc::new(ConnectionHline {
                base: result.between(&result.diamond1, &result.diamond2),
                parent: Rc::clone(&result),
                end_inlink_color: else_color,
            }));
        }
        let out_color = then_block.get_out_link_rendering().get_rainbow_or(arrow_color);
        conns.push(Rc::new(ConnectionOut {
            base: result.between(&result.then_block, &result.diamond2),
            parent: Rc::clone(&result),
            arrow_color: out_color,
        }));
        ftile_utils::add_connections(result, conns)
    }

    fn between(&self, tile1: &Rc<dyn Ftile>, tile2: &Rc<dyn Ftile>) -> AbstractConnection {
        AbstractConnection::new(Some(Rc::clone(tile1)), Some(Rc::clone(tile2)))
    }

    fn connection_else(self: &Rc<Self>, end_inlink_color: Rainbow, kind: ElseKind) -> Rc<dyn Connection> {
        Rc::new(ConnectionElse {
            base: self.between(&self.diamond1, &self.diamond2),
            parent: Rc::clone(self),
            end_inlink_color,
            kind,
        })
    }

    fn get_south_label_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        if let Some(diamond1) = downcast::<FtileDiamondInside>(self.diamond1.as_ref()) {
            return diamond1.get_south_label_height(string_bounder);
        }
        downcast::<FtileDiamond>(self.diamond1.as_ref())
            .map_or(0.0, |diamond1| diamond1.get_south_label_height(string_bounder))
    }

    fn get_east_label_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        if let Some(diamond1) = downcast::<FtileDiamondInside>(self.diamond1.as_ref()) {
            return diamond1.get_east_label_width(string_bounder);
        }
        downcast::<FtileDiamond>(self.diamond1.as_ref())
            .map_or(0.0, |diamond1| diamond1.get_east_label_width(string_bounder))
    }

    fn get_additional_width(&self, stop: &dyn Ftile, string_bounder: &dyn StringBounder) -> f64 {
        let stop_width = stop.calculate_dimension(string_bounder).get_width();
        let val1 = self.get_east_label_width(string_bounder);
        stop_width.max(val1 + stop_width / 2.0)
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let geo_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let geo_then = self.then_block.calculate_dimension(string_bounder);
        let geo_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let geo = geo_diamond1
            .append_bottom(geo_then)
            .append_bottom(geo_diamond2);
        let height = geo.get_height()
            + 3.0 * HEXAGON_HALF_SIZE
            + HEXAGON_HALF_SIZE.max(self.get_south_label_height(string_bounder));
        let mut width = geo.get_width() + HEXAGON_HALF_SIZE;
        if let Some(stop) = &self.optional_stop {
            width += stop.calculate_dimension(string_bounder).get_width()
                + self.get_additional_width(stop.as_ref(), string_bounder);
        }
        let result = FtileGeometry::with_out(
            width,
            height,
            geo.get_left(),
            geo_diamond1.get_in_y(),
            height,
        );
        if !geo_then.has_point_out() && self.optional_stop.is_some() {
            return result.without_point_out();
        }
        result
    }

    fn get_translate_for_then(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_then = self.then_block.calculate_dimension(string_bounder);
        let y = dim_diamond1.get_height()
            + (dim_total.get_height()
                - dim_diamond1.get_height()
                - dim_diamond2.get_height()
                - dim_then.get_height())
                / 2.0;
        let x = dim_total.get_left() - dim_then.get_left();
        UTranslate::new(x, y)
    }

    fn get_translate_diamond1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(dim_total.get_left() - dim_diamond1.get_left(), 0.0)
    }

    fn get_translate_optional_stop(
        &self,
        stop: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_stop = stop.calculate_dimension(string_bounder);
        let label_north = dim_diamond1.get_in_y();
        let y1 = label_north
            + (dim_diamond1.get_height() - label_north - dim_stop.get_height()) / 2.0;
        let x1 = dim_total.get_left() - dim_diamond1.get_left()
            + dim_diamond1.get_width()
            + self.get_additional_width(stop, string_bounder);
        UTranslate::new(x1, y1)
    }

    fn get_translate_diamond2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_left() - dim_diamond2.get_left(),
            dim_total.get_height() - dim_diamond2.get_height(),
        )
    }

    /// The point `x` across a diamond, halfway between where arrows enter and leave it.
    fn middle_of(
        diamond: &dyn Ftile,
        translate: UTranslate,
        x: f64,
        string_bounder: &dyn StringBounder,
    ) -> XPoint2D {
        let dim = diamond.calculate_dimension(string_bounder);
        let half = (dim.get_out_y() - dim.get_in_y()) / 2.0;
        translate.get_translated(XPoint2D::new(x, dim.get_in_y() + half))
    }

    fn arrow_down(&self) -> UPolygon {
        self.skin_param().arrows().as_to_down()
    }
}

impl Swimable for FtileIfDown {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = self.then_block.get_swimlanes();
        result.insert(self.get_swimlane_in());
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.diamond1.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        if self.optional_stop.is_none() {
            return self.get_swimlane_in();
        }
        self.then_block.get_swimlane_out()
    }
}

impl Ftile for FtileIfDown {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn get_translate_for(&self, child: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        if same(child, self.then_block.as_ref()) {
            return self.get_translate_for_then(string_bounder);
        }
        if same(child, self.diamond1.as_ref()) {
            return self.get_translate_diamond1(string_bounder);
        }
        if let Some(stop) = &self.optional_stop
            && same(child, stop.as_ref())
        {
            return self.get_translate_optional_stop(stop.as_ref(), string_bounder);
        }
        if same(child, self.diamond2.as_ref()) {
            return self.get_translate_diamond2(string_bounder);
        }
        UTranslate::default()
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        let mut result = vec![
            Rc::clone(&self.then_block),
            Rc::clone(&self.diamond1),
            Rc::clone(&self.diamond2),
        ];
        result.extend(self.optional_stop.clone());
        result
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.apply(self.get_translate_for_then(string_bounder))
            .draw(&self.then_block);
        ug.apply(self.get_translate_diamond1(string_bounder))
            .draw(&self.diamond1);
        match &self.optional_stop {
            None => ug
                .apply(self.get_translate_diamond2(string_bounder))
                .draw(&self.diamond2),
            Some(stop) => ug
                .apply(self.get_translate_optional_stop(stop.as_ref(), string_bounder))
                .draw(stop),
        }
    }
}

/// Implements the tile accessors of a connection through its `base` field.
macro_rules! connection_tiles {
    () => {
        fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
            self.base.get_ftile1()
        }

        fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
            self.base.get_ftile2()
        }
    };
}

/// From the diamond to the stop beside it.
struct ConnectionHorizontal {
    parent: Rc<FtileIfDown>,
    base: AbstractConnection,
    color: Rainbow,
}

impl Connection for ConnectionHorizontal {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let Some(stop) = &parent.optional_stop else {
            return;
        };
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let p1 = parent
            .get_translate_diamond1(string_bounder)
            .get_translated(XPoint2D::new(
                dim_diamond1.get_width(),
                f64::midpoint(dim_diamond1.get_in_y(), dim_diamond1.get_out_y()),
            ));
        let dim_stop = stop.calculate_dimension(string_bounder);
        let p2 = parent
            .get_translate_optional_stop(stop.as_ref(), string_bounder)
            .get_translated(XPoint2D::new(0.0, dim_stop.get_height() / 2.0));
        let mut snake = Snake::create_with_end(
            parent.skin_param(),
            self.color.clone(),
            parent.skin_param().arrows().as_to_right(),
        );
        snake.add_point_at(p1);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }
}

/// From the diamond down into the branch.
struct ConnectionIn {
    parent: Rc<FtileIfDown>,
    base: AbstractConnection,
    arrow_color: Rainbow,
}

impl ConnectionIn {
    fn points(&self, string_bounder: &dyn StringBounder) -> (XPoint2D, XPoint2D) {
        let parent = &self.parent;
        let p1 = parent.get_translate_diamond1(string_bounder).get_translated(
            parent
                .diamond1
                .calculate_dimension(string_bounder)
                .get_point_out(),
        );
        let p2 = parent.get_translate_for_then(string_bounder).get_translated(
            parent
                .then_block
                .calculate_dimension(string_bounder)
                .get_point_in(),
        );
        (p1, p2)
    }
}

impl Connection for ConnectionIn {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let (p1, p2) = self.points(ug.string_bounder());
        let mut snake = Snake::create_with_end(
            self.parent.skin_param(),
            self.arrow_color.clone(),
            self.parent.arrow_down(),
        );
        snake.add_point_at(p1);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionIn {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let (p1, p2) = self.points(ug.string_bounder());
        draw_translated_down(
            &self.parent,
            &self.arrow_color,
            translate1.get_translated(p1),
            translate2.get_translated(p2),
            ug,
        );
    }
}

/// An arrow down from `mp1a` to `mp2b`, in lanes apart: it crosses at mid-height.
fn draw_translated_down(
    parent: &FtileIfDown,
    color: &Rainbow,
    mp1a: XPoint2D,
    mp2b: XPoint2D,
    ug: &UGraphic,
) {
    let mut snake = Snake::create_with_end(parent.skin_param(), color.clone(), parent.arrow_down());
    let middle = f64::midpoint(mp1a.y, mp2b.y);
    snake.add_point_at(mp1a);
    snake.add_point(mp1a.x, middle);
    snake.add_point(mp2b.x, middle);
    snake.add_point_at(mp2b);
    ug.draw(&snake);
}

/// From the branch down to the end of the conditional.
struct ConnectionOut {
    parent: Rc<FtileIfDown>,
    base: AbstractConnection,
    arrow_color: Rainbow,
}

impl ConnectionOut {
    fn get_p1(&self, string_bounder: &dyn StringBounder) -> Option<XPoint2D> {
        let parent = &self.parent;
        let geo = parent.then_block.calculate_dimension(string_bounder);
        geo.has_point_out().then(|| {
            parent
                .get_translate_for_then(string_bounder)
                .get_translated(geo.get_point_out())
        })
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let parent = &self.parent;
        parent.get_translate_diamond2(string_bounder).get_translated(
            parent
                .diamond2
                .calculate_dimension(string_bounder)
                .get_point_in(),
        )
    }
}

impl Connection for ConnectionOut {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let Some(p1) = self.get_p1(string_bounder) else {
            return;
        };
        let parent = &self.parent;
        let mut snake = Snake::create_with_end(
            parent.skin_param(),
            self.arrow_color.clone(),
            parent.arrow_down(),
        );
        snake.add_point_at(p1);
        match parent.condition_end_style {
            ConditionEndStyle::Diamond => snake.add_point_at(self.get_p2(string_bounder)),
            ConditionEndStyle::Hline => {
                let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
                snake.add_point_at(FtileIfDown::middle_of(
                    parent.diamond2.as_ref(),
                    parent.get_translate_diamond2(string_bounder),
                    dim_diamond2.get_width(),
                    string_bounder,
                ));
            }
        }
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionOut {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let Some(p1) = self.get_p1(string_bounder) else {
            return;
        };
        let p2 = self.get_p2(string_bounder);
        draw_translated_down(
            &self.parent,
            &self.arrow_color,
            translate1.get_translated(p1),
            translate2.get_translated(p2),
            ug,
        );
    }
}

/// The ways the other branch goes round (PlantUML's `ConnectionElse1`, `ConnectionElse2` and its
/// subclasses `ConnectionElseHline` and `ConnectionElseNoDiamond`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ElseKind {
    /// Round the left, into the left of the second diamond.
    Else1,
    /// Round the right, into the right of the second diamond.
    Else2,
    /// Round the right, down to the line ending the conditional.
    Hline,
    /// Round the right, to where the conditional is left: the branch ends in a stop.
    NoDiamond,
}

struct ConnectionElse {
    parent: Rc<FtileIfDown>,
    base: AbstractConnection,
    end_inlink_color: Rainbow,
    kind: ElseKind,
}

impl ConnectionElse {
    fn get_p2(&self, x: f64, string_bounder: &dyn StringBounder) -> XPoint2D {
        let parent = &self.parent;
        if self.kind == ElseKind::NoDiamond {
            return parent.calculate_dimension(string_bounder).get_point_out();
        }
        FtileIfDown::middle_of(
            parent.diamond2.as_ref(),
            parent.get_translate_diamond2(string_bounder),
            x,
            string_bounder,
        )
    }

    /// Where the other branch goes down: beyond the diamond and the branch.
    fn xmax(&self, x1: f64, string_bounder: &dyn StringBounder) -> f64 {
        let parent = &self.parent;
        let then_geom = parent.then_block.calculate_dimension(string_bounder);
        (x1 + HEXAGON_HALF_SIZE)
            .max(parent.get_translate_for_then(string_bounder).dx + then_geom.get_width())
    }
}

impl Connection for ConnectionElse {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let left = self.kind == ElseKind::Else1;
        let x_diamond1 = if left {
            0.0
        } else {
            parent
                .diamond1
                .calculate_dimension(string_bounder)
                .get_width()
        };
        let p1 = FtileIfDown::middle_of(
            parent.diamond1.as_ref(),
            parent.get_translate_diamond1(string_bounder),
            x_diamond1,
            string_bounder,
        );
        if !parent.calculate_dimension(string_bounder).has_point_out() {
            return;
        }
        let x_diamond2 = if left {
            0.0
        } else {
            parent
                .diamond2
                .calculate_dimension(string_bounder)
                .get_width()
        };
        let p2 = self.get_p2(x_diamond2, string_bounder);
        let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
        let skin_param = parent.skin_param();
        let arrows = skin_param.arrows();
        let empty = UShape::Empty(XDimension2D::new(5.0, HEXAGON_HALF_SIZE));
        let (snake, empty_at) = match self.kind {
            ElseKind::Else1 => {
                let xmin =
                    (x1 - HEXAGON_HALF_SIZE).min(parent.get_translate_for_then(string_bounder).dx);
                let mut snake = Snake::create_with_end(
                    skin_param,
                    self.end_inlink_color.clone(),
                    arrows.as_to_right(),
                )
                .emphasize_direction(Direction::Down);
                snake.add_point(x1, y1);
                snake.add_point(xmin, y1);
                snake.add_point(xmin, y2);
                snake.add_point(x2, y2);
                (snake, x2)
            }
            ElseKind::Else2 | ElseKind::NoDiamond => {
                let xmax = self.xmax(x1, string_bounder);
                let mut snake = Snake::create_with_end(
                    skin_param,
                    self.end_inlink_color.clone(),
                    arrows.as_to_left(),
                )
                .emphasize_direction(Direction::Down);
                snake.add_point(x1, y1);
                snake.add_point(xmax, y1);
                snake.add_point(xmax, y2);
                snake.add_point(x2, y2);
                (snake, x2)
            }
            ElseKind::Hline => {
                let xmax = self.xmax(x1, string_bounder);
                let mut snake = Snake::create_with_end(
                    skin_param,
                    self.end_inlink_color.clone(),
                    arrows.as_to_down(),
                );
                snake.add_point(x1, y1);
                snake.add_point(xmax, y1);
                snake.add_point(xmax, y2);
                (snake, xmax)
            }
        };
        ug.apply(UTranslate::new(empty_at, y2 - HEXAGON_HALF_SIZE))
            .draw(&empty);
        ug.draw(&snake);
    }
}

/// With `conditionEndStyle hline`: the line ending the conditional, from the other branch to the south of
/// the second diamond.
struct ConnectionHline {
    parent: Rc<FtileIfDown>,
    base: AbstractConnection,
    end_inlink_color: Rainbow,
}

impl Connection for ConnectionHline {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let p1 = FtileIfDown::middle_of(
            parent.diamond1.as_ref(),
            parent.get_translate_diamond1(string_bounder),
            dim_diamond1.get_width(),
            string_bounder,
        );
        if !parent.calculate_dimension(string_bounder).has_point_out() {
            return;
        }
        let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
        let translate_diamond2 = parent.get_translate_diamond2(string_bounder);
        let p2 = FtileIfDown::middle_of(
            parent.diamond2.as_ref(),
            translate_diamond2,
            dim_diamond2.get_width(),
            string_bounder,
        );
        let p3 = translate_diamond2.get_translated(XPoint2D::new(
            dim_diamond2.get_width(),
            dim_diamond2.get_out_y(),
        ));
        let then_geom = parent.then_block.calculate_dimension(string_bounder);
        let xmax = (p1.x + HEXAGON_HALF_SIZE)
            .max(parent.get_translate_for_then(string_bounder).dx + then_geom.get_width());
        let mut snake = Snake::create(parent.skin_param(), self.end_inlink_color.clone())
            .with_merge(MergeStrategy::None);
        snake.add_point(xmax, p2.y);
        snake.add_point_at(p2);
        snake.add_point_at(p3);
        ug.draw(&snake);
    }
}
