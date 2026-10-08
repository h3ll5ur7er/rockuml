//! A `while` loop (PlantUML's `FtileWhile`): the test above the body, the arrow back from the body to the
//! test on the right, the way out on the left. A `stop` or `end` right after `endwhile` ends the way out
//! inside the tile, as its `special_out`.

use std::rc::Rc;

use super::display_text;
use crate::color::HColor;
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::direction::Direction;
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::vertical::{FtileDiamond, FtileDiamondInside, FtileDiamondSquare};
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, ConnectionTranslatable, Ftile, FtileFactory,
    FtileGeometry, MergeStrategy, Snake, Swimable, ftile_utils, same,
};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UChange, UGraphic};
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::svek::ConditionStyle;

pub(crate) struct FtileWhile {
    base: AbstractFtile,
    while_block: Rc<dyn Ftile>,
    diamond1: Rc<dyn Ftile>,
    special_out: Option<Rc<dyn Ftile>>,
    backward: Option<Rc<dyn Ftile>>,
    /// The label of the arrow back to the test.
    back1: Rc<dyn TextBlock>,
}

/// The colours and fonts of a `while`, read from the styles by `FtileFactoryDelegatorWhile`.
pub(crate) struct WhileStyle {
    pub(crate) border_color: HColor,
    pub(crate) back_color: HColor,
    pub(crate) arrow_color: Rainbow,
    pub(crate) font_arrow: FontConfiguration,
    pub(crate) condition_style: ConditionStyle,
    pub(crate) fc_test: FontConfiguration,
}

impl WhileStyle {
    /// An arrow label.
    fn arrow_text(&self, display: Option<&Display>, skin_param: &SkinParam) -> Rc<dyn TextBlock> {
        display_text(
            display,
            &self.font_arrow,
            HorizontalAlignment::Left,
            skin_param,
            CreoleMode::Full,
        )
    }
}

/// The test, drawn as `skinparam conditionStyle` says, `yes` above it and `out` left of it.
fn create_diamond1(
    style: &WhileStyle,
    skin_param: &Rc<SkinParam>,
    swimlane: Option<SwimlaneId>,
    test_tb: Rc<dyn TextBlock>,
    yes_tb: Rc<dyn TextBlock>,
    out_tb: Rc<dyn TextBlock>,
) -> Rc<dyn Ftile> {
    let (back_color, border_color) = (style.back_color.clone(), style.border_color.clone());
    match style.condition_style {
        ConditionStyle::InsideHexagon => Rc::new(
            FtileDiamondInside::new(
                test_tb,
                skin_param.clone(),
                back_color,
                border_color,
                swimlane,
            )
            .with_north(yes_tb)
            .with_west(out_tb),
        ),
        ConditionStyle::InsideDiamond => Rc::new(
            FtileDiamondSquare::new(
                test_tb,
                skin_param.clone(),
                back_color,
                border_color,
                swimlane,
            )
            .with_north(yes_tb)
            .with_west(out_tb),
        ),
        ConditionStyle::EmptyDiamond => Rc::new(
            FtileDiamond::new(skin_param.clone(), back_color, border_color, swimlane)
                .with_north(test_tb)
                .with_south(yes_tb)
                .with_west(out_tb),
        ),
    }
}

impl FtileWhile {
    /// The loop around `while_block`, with its arrows. `special_out` is the tile of the `stop` or `end`
    /// right after `endwhile`, built with `ftile_factory`.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's FtileWhile.create")]
    pub(crate) fn create(
        out_color: &LinkRendering,
        swimlane: Option<SwimlaneId>,
        while_block: Rc<dyn Ftile>,
        test: &Display,
        yes: Option<&Display>,
        style: &WhileStyle,
        ftile_factory: &dyn FtileFactory,
        special_out: Option<Rc<dyn Ftile>>,
        backward: Option<Rc<dyn Ftile>>,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
    ) -> Rc<dyn Ftile> {
        let skin_param = ftile_factory.skin_param();
        let test_tb: Rc<dyn TextBlock> = if test.is_white() {
            Rc::new(TextBlockEmpty::default())
        } else {
            display_text(
                Some(test),
                &style.fc_test,
                while_block
                    .skin_param()
                    .get_default_text_alignment(HorizontalAlignment::Left),
                skin_param,
                CreoleMode::Full,
            )
        };
        let diamond1 = create_diamond1(
            style,
            skin_param,
            swimlane,
            test_tb,
            style.arrow_text(yes, skin_param),
            style.arrow_text(out_color.display.as_ref(), skin_param),
        );

        let dim = while_block.calculate_dimension(ftile_factory.get_string_bounder());
        let result = Rc::new(Self {
            base: AbstractFtile::new(skin_param.clone()),
            while_block,
            diamond1,
            special_out,
            backward,
            back1: style.arrow_text(incoming1.display.as_ref(), skin_param),
        });

        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        if dim.get_width() == 0.0 || dim.get_height() == 0.0 {
            conns.push(Rc::new(ConnectionBackEmpty {
                connection: result.connection(&result.diamond1, Some(&result.diamond1)),
                end_inlink_color: incoming1.rainbow.clone(),
            }));
        } else {
            result.add_connections_in_and_back(&mut conns, style, incoming1, incoming2);
        }
        let diamond1 = &result.diamond1;
        if result.special_out.is_some() {
            conns.push(Rc::new(ConnectionOutSpecial {
                connection: result.connection(diamond1, result.special_out.as_ref()),
                after_endwhile_color: out_color.rainbow.clone(),
            }));
        } else {
            conns.push(Rc::new(ConnectionOut {
                connection: result.connection(diamond1, None),
                after_endwhile_color: out_color.rainbow.clone(),
            }));
        }
        ftile_utils::add_connections(result, conns)
    }

    fn add_connections_in_and_back(
        self: &Rc<Self>,
        conns: &mut Vec<Rc<dyn Connection>>,
        style: &WhileStyle,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
    ) {
        let (while_block, diamond1) = (&self.while_block, &self.diamond1);
        conns.push(Rc::new(ConnectionIn {
            connection: self.connection(diamond1, Some(while_block)),
            arrow_color: while_block
                .get_in_link_rendering()
                .get_rainbow_or(&style.arrow_color),
        }));
        match &self.backward {
            None => conns.push(Rc::new(ConnectionBackSimple {
                connection: self.connection(while_block, Some(diamond1)),
                end_inlink_color: incoming1.rainbow.clone(),
                back: self.back1.clone(),
            })),
            Some(backward) => {
                conns.push(Rc::new(ConnectionBackBackward1 {
                    connection: self.connection(while_block, Some(backward)),
                    end_inlink_color: incoming1.rainbow.clone(),
                    back: self.back1.clone(),
                }));
                conns.push(Rc::new(ConnectionBackBackward2 {
                    connection: self.connection(backward, Some(diamond1)),
                    end_inlink_color: incoming2.rainbow.clone(),
                    back: style.arrow_text(incoming2.display.as_ref(), self.skin_param()),
                }));
            }
        }
    }

    fn connection(
        self: &Rc<Self>,
        ftile1: &Rc<dyn Ftile>,
        ftile2: Option<&Rc<dyn Ftile>>,
    ) -> WhileConnection {
        WhileConnection {
            base: AbstractConnection::new(Some(ftile1.clone()), ftile2.cloned()),
            tile: self.clone(),
        }
    }

    fn get_translate_backward(
        &self,
        backward: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_backward = backward.calculate_dimension(string_bounder);
        let x = dim_total.get_width() - dim_backward.get_width();
        let y = (dim_total.get_height() - dim_backward.get_height()) / 2.0;
        UTranslate::new(x, y)
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let geo_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let geo_while = self.while_block.calculate_dimension(string_bounder);
        let geo = geo_diamond1.append_bottom(geo_while);
        let height = geo.get_height()
            + 4.0 * HEXAGON_HALF_SIZE
            + self.get_supp_height_for_label(string_bounder);
        let dx = 2.0 * HEXAGON_HALF_SIZE;
        let backward_width = self.backward.as_ref().map_or(0.0, |backward| {
            backward.calculate_dimension(string_bounder).get_width()
        });
        let x_delta = self.x_delta_because_special(string_bounder);
        FtileGeometry::with_out(
            x_delta + geo.get_width() + dx + HEXAGON_HALF_SIZE + backward_width,
            height,
            x_delta + geo.get_left() + dx,
            geo_diamond1.get_in_y(),
            height,
        )
    }

    fn get_supp_height_for_label(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.back1.calculate_dimension(string_bounder).height
    }

    fn x_delta_because_special(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.special_out.as_ref().map_or(0.0, |special_out| {
            special_out.calculate_dimension(string_bounder).get_width()
        })
    }

    fn get_translate_for_while(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_while = self.while_block.calculate_dimension(string_bounder);
        let y = dim_diamond1.get_height()
            + (dim_total.get_height()
                - dim_diamond1.get_height()
                - dim_while.get_height()
                - self.get_supp_height_for_label(string_bounder))
                / 2.0;
        let x = dim_total.get_left() - dim_while.get_left();
        UTranslate::new(x, y)
    }

    fn get_translate_diamond1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(dim_total.get_left() - dim_diamond1.get_left(), 0.0)
    }

    fn get_translate_for_special(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let half = (dim_diamond1.get_out_y() - dim_diamond1.get_in_y()) / 2.0;
        let y1 = (3.0 * half).max(4.0 * HEXAGON_HALF_SIZE);
        let x_while = self.get_translate_for_while(string_bounder).dx - HEXAGON_HALF_SIZE;
        let x_diamond = self.get_translate_diamond1(string_bounder).dx;
        let x1 = x_while.min(x_diamond) - self.x_delta_because_special(string_bounder);
        UTranslate::new(x1, y1)
    }

    /// Where the arrow back from the body leaves it, if the body has a way out.
    fn get_while_out(&self, string_bounder: &dyn StringBounder) -> Option<XPoint2D> {
        let geo = self.while_block.calculate_dimension(string_bounder);
        geo.has_point_out().then(|| {
            self.get_translate_for_while(string_bounder)
                .get_translated(geo.get_point_out())
        })
    }

    /// Below the body.
    fn get_bottom(&self, string_bounder: &dyn StringBounder) -> f64 {
        let geo = self.while_block.calculate_dimension(string_bounder);
        self.get_translate_for_while(string_bounder).dy + geo.get_height()
    }

    /// Where the test's top left corner is.
    fn get_diamond1_origin(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        self.get_translate_diamond1(string_bounder)
            .get_translated(XPoint2D::new(0.0, 0.0))
    }

    /// Halfway down the test's shape, below its label.
    fn get_diamond1_middle_y(&self, origin: XPoint2D, string_bounder: &dyn StringBounder) -> f64 {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let half = (dim_diamond1.get_out_y() - dim_diamond1.get_in_y()) / 2.0;
        origin.y + dim_diamond1.get_in_y() + half
    }

    fn arrow(&self, color: &Rainbow, direction: Direction) -> Snake {
        Snake::create_with_end(
            self.skin_param(),
            color.clone(),
            self.skin_param().arrows().as_to(direction),
        )
    }
}

impl Swimable for FtileWhile {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = self.while_block.get_swimlanes();
        result.insert(self.get_swimlane_in());
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.diamond1.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.get_swimlane_in()
    }
}

impl Ftile for FtileWhile {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.while_block.as_ref()) {
            return self.get_translate_for_while(string_bounder);
        }
        if same(child, self.diamond1.as_ref()) {
            return self.get_translate_diamond1(string_bounder);
        }
        UTranslate::default()
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        let mut children = vec![self.while_block.clone(), self.diamond1.clone()];
        children.extend(self.special_out.clone());
        children
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.apply(self.get_translate_for_while(string_bounder))
            .draw(&self.while_block);
        ug.apply(self.get_translate_diamond1(string_bounder))
            .draw(&self.diamond1);
        if let Some(special_out) = &self.special_out {
            ug.apply(self.get_translate_for_special(string_bounder))
                .draw(special_out);
        }
        if let Some(backward) = &self.backward {
            ug.apply(self.get_translate_backward(backward.as_ref(), string_bounder))
                .draw(backward);
        }
    }
}

/// What the connections of a `while` share: the tiles they join and the loop they belong to (PlantUML's
/// inner classes reach the loop's fields).
struct WhileConnection {
    base: AbstractConnection,
    tile: Rc<FtileWhile>,
}

impl WhileConnection {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }
}

/// The room left below the arrow back to the test, which compression keeps.
fn draw_room_below(ug: &UGraphic, x: f64, y: f64) {
    ug.apply(UTranslate::new(x, y))
        .draw(&UShape::Empty(XDimension2D::new(5.0, HEXAGON_HALF_SIZE)));
}

/// From the test down into the body.
struct ConnectionIn {
    connection: WhileConnection,
    arrow_color: Rainbow,
}

impl ConnectionIn {
    fn get_p1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = &self.connection.tile;
        tile.get_translate_diamond1(string_bounder).get_translated(
            tile.diamond1
                .calculate_dimension(string_bounder)
                .get_point_out(),
        )
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = &self.connection.tile;
        tile.get_translate_for_while(string_bounder).get_translated(
            tile.while_block
                .calculate_dimension(string_bounder)
                .get_point_in(),
        )
    }
}

impl Connection for ConnectionIn {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let mut snake = self
            .connection
            .tile
            .arrow(&self.arrow_color, Direction::Down);
        snake.add_point_at(self.get_p1(string_bounder));
        snake.add_point_at(self.get_p2(string_bounder));
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionIn {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let p1 = self.get_p1(string_bounder);
        let p2 = self.get_p2(string_bounder);
        let mut snake = self
            .connection
            .tile
            .arrow(&self.arrow_color, Direction::Down)
            .with_merge(MergeStrategy::Limited);
        let start = translate1.get_translated(p1);
        let end = translate2.get_translated(p2);
        let middle = f64::midpoint(start.y, end.y);
        snake.add_point_at(start);
        snake.add_point(start.x, middle);
        snake.add_point(end.x, middle);
        snake.add_point_at(end);
        ug.draw(&snake);
    }
}

/// From the end of the body, around its right, back into the test.
struct ConnectionBackSimple {
    connection: WhileConnection,
    end_inlink_color: Rainbow,
    back: Rc<dyn TextBlock>,
}

impl Connection for ConnectionBackSimple {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let dim_total = tile.calculate_dimension(string_bounder);
        let Some(p1) = tile.get_while_out(string_bounder) else {
            return;
        };
        let p2 = tile.get_diamond1_origin(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);

        let (x1, y1) = (p1.x, p1.y);
        let x2 = p2.x + dim_diamond1.get_width();
        let y2 = tile.get_diamond1_middle_y(p2, string_bounder);

        let mut snake = tile
            .arrow(&self.end_inlink_color, Direction::Left)
            .emphasize_direction(Direction::Up)
            .with_label_vertical(Some(self.back.clone()), VerticalAlignment::Bottom);
        snake.add_point(x1, y1);
        let y1bis = y1.max(tile.get_bottom(string_bounder)) + HEXAGON_HALF_SIZE;
        snake.add_point(x1, y1bis);
        let xx = dim_total.get_width();
        snake.add_point(xx, y1bis);
        snake.add_point(xx, y2);
        snake.add_point(x2, y2);

        ug.draw(&snake);
        draw_room_below(ug, x1, y1bis);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionBackSimple {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let mut snake = tile
            .arrow(&self.end_inlink_color, Direction::Left)
            .with_merge(MergeStrategy::Limited);
        let dim_total = tile.calculate_dimension(string_bounder);
        let Some(ap1) = tile.get_while_out(string_bounder) else {
            return;
        };
        let ap2 = tile.get_diamond1_origin(string_bounder);
        let p1 = translate1.get_translated(ap1);
        let p2 = translate2.get_translated(ap2);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);

        let (x1, y1) = (p1.x, p1.y);
        let x2 = p2.x + dim_diamond1.get_width();
        let y2 = tile.get_diamond1_middle_y(p2, string_bounder);

        snake.add_point(x1, y1);
        snake.add_point(x1, y1 + HEXAGON_HALF_SIZE);
        let xx = translate1.dx.max(translate2.dx) + dim_total.get_width();
        snake.add_point(xx, y1 + HEXAGON_HALF_SIZE);
        snake.add_point(xx, y2);
        snake.add_point(x2, y2);

        ug.draw(&snake);
        draw_room_below(ug, x1, y1 + HEXAGON_HALF_SIZE);

        let color = self.end_inlink_color.get_color().clone();
        let ug = ug.apply(color.clone()).apply(UChange::Background(color));
        ug.apply(UTranslate::new(xx, f64::midpoint(y1, y2)))
            .draw(&UShape::Polygon(tile.skin_param().arrows().as_to_up()));
    }
}

/// From the end of the body, around its right, up into the `backward` activity.
struct ConnectionBackBackward1 {
    connection: WhileConnection,
    end_inlink_color: Rainbow,
    back: Rc<dyn TextBlock>,
}

impl Connection for ConnectionBackBackward1 {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let Some(p1) = tile.get_while_out(string_bounder) else {
            return;
        };
        let Some(backward) = &tile.backward else {
            return;
        };
        let dim = backward.calculate_dimension(string_bounder);
        let p2 = tile
            .get_translate_backward(backward.as_ref(), string_bounder)
            .get_translated(XPoint2D::new(dim.get_left(), dim.get_out_y()));
        let (x1, y1) = (p1.x, p1.y);
        let (x2, y2) = (p2.x, p2.y);

        let mut snake = tile
            .arrow(&self.end_inlink_color, Direction::Up)
            .with_label_vertical(Some(self.back.clone()), VerticalAlignment::Bottom);
        snake.add_point(x1, y1);
        let y1bis = y1.max(tile.get_bottom(string_bounder)) + HEXAGON_HALF_SIZE;
        snake.add_point(x1, y1bis);
        snake.add_point(x2, y1bis);
        snake.add_point(x2, y2);

        ug.draw(&snake);
        draw_room_below(ug, x1, y1bis);
    }
}

/// From the `backward` activity left into the test.
struct ConnectionBackBackward2 {
    connection: WhileConnection,
    end_inlink_color: Rainbow,
    back: Rc<dyn TextBlock>,
}

impl Connection for ConnectionBackBackward2 {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let Some(backward) = &tile.backward else {
            return;
        };
        let mut snake = tile
            .arrow(&self.end_inlink_color, Direction::Left)
            .with_label(
                Some(self.back.clone()),
                self.connection.base.arrow_horizontal_alignment(),
            );

        let dim = backward.calculate_dimension(string_bounder);
        let p1 = tile
            .get_translate_backward(backward.as_ref(), string_bounder)
            .get_translated(XPoint2D::new(dim.get_left(), dim.get_in_y()));
        let p2 = tile.get_diamond1_origin(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);

        let (x1, y1) = (p1.x, p1.y);
        let x2 = p2.x + dim_diamond1.get_width();
        let y2 = tile.get_diamond1_middle_y(p2, string_bounder);

        snake.add_point(x1, y1);
        snake.add_point(x1, y2);
        snake.add_point(x2, y2);

        ug.draw(&snake);
    }
}

/// Around an empty body, from the test back into it.
struct ConnectionBackEmpty {
    connection: WhileConnection,
    end_inlink_color: Rainbow,
}

impl Connection for ConnectionBackEmpty {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let mut snake = tile
            .arrow(&self.end_inlink_color, Direction::Left)
            .emphasize_direction(Direction::Up);
        let dim_total = tile.calculate_dimension(string_bounder);
        let p1 = tile.get_translate_diamond1(string_bounder).get_translated(
            tile.diamond1
                .calculate_dimension(string_bounder)
                .get_point_out(),
        );
        let p2 = tile.get_diamond1_origin(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);

        let (x1, y1) = (p1.x, p1.y);
        let x2 = p2.x + dim_diamond1.get_width();
        let y2 = tile.get_diamond1_middle_y(p2, string_bounder);

        snake.add_point(x1, y1);
        let y1bis = y1.max(tile.get_bottom(string_bounder)) + HEXAGON_HALF_SIZE;
        snake.add_point(x1, y1bis);
        let xx = dim_total.get_width();
        snake.add_point(xx, y1bis);
        snake.add_point(xx, y2);
        snake.add_point(x2, y2);

        ug.draw(&snake);
        draw_room_below(ug, x1, y1bis);
    }
}

/// From the test, around the left of the body, down to the bottom of the loop.
struct ConnectionOut {
    connection: WhileConnection,
    after_endwhile_color: Rainbow,
}

impl Connection for ConnectionOut {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let mut snake = Snake::create(tile.skin_param(), self.after_endwhile_color.clone())
            .with_merge(MergeStrategy::Limited)
            .emphasize_direction(Direction::Down);

        let p1 = tile.get_diamond1_origin(string_bounder);
        let dim_total = tile.calculate_dimension(string_bounder);
        let p2 = XPoint2D::new(dim_total.get_left(), dim_total.get_height());

        let x1 = p1.x;
        let y1 = tile.get_diamond1_middle_y(p1, string_bounder);
        let (x2, y2) = (p2.x, p2.y);

        snake.add_point(x1, y1);
        snake.add_point(HEXAGON_HALF_SIZE, y1);
        snake.add_point(HEXAGON_HALF_SIZE, y2);
        ug.draw(&snake);

        let mut snake2 = Snake::create(tile.skin_param(), self.after_endwhile_color.clone());
        snake2.add_point(HEXAGON_HALF_SIZE, y2);
        snake2.add_point(x2, y2);
        ug.draw(&snake2);
    }
}

/// From the test, left and down into the `stop` or `end` after the loop.
struct ConnectionOutSpecial {
    connection: WhileConnection,
    after_endwhile_color: Rainbow,
}

impl Connection for ConnectionOutSpecial {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let Some(special_out) = &tile.special_out else {
            return;
        };
        let mut snake = tile.arrow(&self.after_endwhile_color, Direction::Down);

        let p1 = tile.get_diamond1_origin(string_bounder);
        let p2 = tile
            .get_translate_for_special(string_bounder)
            .get_translated(
                special_out
                    .calculate_dimension(string_bounder)
                    .get_point_in(),
            );

        let x1 = p1.x;
        let y1 = tile.get_diamond1_middle_y(p1, string_bounder);
        let (x2, y2) = (p2.x, p2.y);

        snake.add_point(x1, y1);
        snake.add_point(x2, y1);
        snake.add_point(x2, y2);

        ug.draw(&snake);
    }
}
