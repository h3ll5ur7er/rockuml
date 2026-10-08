//! A `repeat` loop (PlantUML's `FtileRepeat`): an entry diamond (or the `repeat :label;` activity) above
//! the body, the test below it, and the arrow from the test back up to the entry.

use std::rc::Rc;

use super::create0_or_empty;
use crate::color::HColor;
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::direction::Direction;
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::vertical::{FtileDiamond, FtileDiamondInside, FtileDiamondSquare};
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, ConnectionTranslatable, Ftile, FtileEmpty,
    FtileGeometry, Snake, Swimable, ftile_utils, same,
};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::svek::ConditionStyle;

pub(crate) struct FtileRepeat {
    base: AbstractFtile,
    repeat: Rc<dyn Ftile>,
    diamond1: Rc<dyn Ftile>,
    diamond2: Rc<dyn Ftile>,
    backward: Option<Rc<dyn Ftile>>,
    /// The test, when written beside an empty diamond; it widens the loop.
    tb_test: Rc<dyn TextBlock>,
}

/// The colours and fonts of a `repeat`, read from the styles by `FtileFactoryDelegatorRepeat`.
pub(crate) struct RepeatStyle {
    pub(crate) border_color: HColor,
    pub(crate) diamond_color1: HColor,
    pub(crate) diamond_color2: HColor,
    pub(crate) arrow_color: Rainbow,
    /// The colour of the arrow out of the body, set before `repeat while`.
    pub(crate) end_repeat_link_color: Rainbow,
    pub(crate) condition_style: ConditionStyle,
    pub(crate) fc_diamond: FontConfiguration,
    pub(crate) fc_arrow: FontConfiguration,
}

impl RepeatStyle {
    /// An arrow label.
    fn arrow_text(
        &self,
        display: Option<&Display>,
        skin_param: &SkinParam,
        mode: CreoleMode,
    ) -> Rc<dyn TextBlock> {
        create0_or_empty(
            display,
            &self.fc_arrow,
            HorizontalAlignment::Left,
            skin_param,
            0.0,
            mode,
        )
    }
}

impl FtileRepeat {
    /// The loop around `repeat`, with its arrows. `entry` is the activity of `repeat :label;`, which
    /// replaces the entry diamond; `no_out` is PlantUML's `isLastOfTheParent()`.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's FtileRepeat.create")]
    pub(crate) fn create(
        swimlane: Option<SwimlaneId>,
        swimlane_out: Option<SwimlaneId>,
        entry: Option<Rc<dyn Ftile>>,
        repeat: Rc<dyn Ftile>,
        test: Option<&Display>,
        yes: Option<&Display>,
        out: Option<&Display>,
        style: &RepeatStyle,
        skin_param: &Rc<SkinParam>,
        backward: Option<Rc<dyn Ftile>>,
        no_out: bool,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
    ) -> Rc<dyn Ftile> {
        let font_configuration1 = if style.condition_style == ConditionStyle::InsideHexagon {
            &style.fc_diamond
        } else {
            &style.fc_arrow
        };
        let tb_test: Rc<dyn TextBlock> = match test {
            Some(test) if !test.is_white() => create0_or_empty(
                Some(test),
                font_configuration1,
                repeat
                    .skin_param()
                    .get_default_text_alignment(HorizontalAlignment::Left),
                skin_param,
                0.0,
                CreoleMode::Full,
            ),
            _ => empty(),
        };
        let yes_tb = style.arrow_text(yes, skin_param, CreoleMode::Full);
        let out_tb = style.arrow_text(out, skin_param, CreoleMode::Full);

        let diamond1 = entry.unwrap_or_else(|| {
            Rc::new(FtileDiamond::new(
                skin_param.clone(),
                style.diamond_color1.clone(),
                style.border_color.clone(),
                swimlane,
            ))
        });

        let (diamond2, tb_test) = create_diamond2(
            style,
            skin_param,
            (swimlane, swimlane_out),
            [tb_test, yes_tb, out_tb],
            no_out && test.is_none(),
            backward_exits_on_left(backward.as_deref(), swimlane_out),
        );

        let result = Rc::new(Self {
            base: AbstractFtile::new(skin_param.clone()),
            repeat,
            diamond1,
            diamond2,
            backward,
            tb_test,
        });
        let conns =
            result.create_connections(style, (swimlane, swimlane_out), incoming1, incoming2);
        ftile_utils::add_connections(result, conns)
    }

    /// The arrows in, back and out; which way back depends on the lanes of the entry (`swimlane`) and
    /// of the test (`swimlane_out`).
    fn create_connections(
        self: &Rc<Self>,
        style: &RepeatStyle,
        (swimlane, swimlane_out): (Option<SwimlaneId>, Option<SwimlaneId>),
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
    ) -> Vec<Rc<dyn Connection>> {
        let skin_param = self.skin_param();
        let (repeat, diamond1, diamond2) = (&self.repeat, &self.diamond1, &self.diamond2);
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let in_link_rendering = repeat.get_in_link_rendering();
        conns.push(Rc::new(ConnectionIn {
            connection: self.connection(diamond1, repeat),
            arrow_color: in_link_rendering.get_rainbow_or(&style.arrow_color),
            tbin: style.arrow_text(
                in_link_rendering.display.as_ref(),
                skin_param,
                CreoleMode::SimpleLine,
            ),
        }));

        let incoming_text = style.arrow_text(
            incoming1.display.as_ref(),
            skin_param,
            CreoleMode::SimpleLine,
        );
        let back_arrow_color = incoming1.get_rainbow_or(&style.arrow_color);
        match &self.backward {
            Some(backward) => {
                conns.push(Rc::new(ConnectionBackBackward1 {
                    connection: self.connection(diamond2, backward),
                    arrow_color: back_arrow_color,
                    tbback: incoming_text,
                }));
                conns.push(Rc::new(ConnectionBackBackward2 {
                    connection: self.connection(backward, diamond1),
                    arrow_color: incoming2.get_rainbow_or(&style.arrow_color),
                    label: style.arrow_text(
                        incoming2.display.as_ref(),
                        skin_param,
                        CreoleMode::Full,
                    ),
                }));
            }
            None if swimlane.is_none() || swimlane == swimlane_out => {
                let connection = self.connection(diamond2, diamond1);
                if swimlane
                    .is_some_and(|lane| lane.is_smaller_than_all_others(&repeat.get_swimlanes()))
                {
                    conns.push(Rc::new(ConnectionBackSimple1 {
                        connection,
                        arrow_color: back_arrow_color,
                        tbback: incoming_text,
                    }));
                } else {
                    conns.push(Rc::new(ConnectionBackSimple2 {
                        connection,
                        arrow_color: back_arrow_color,
                        tbback: incoming_text,
                    }));
                }
            }
            None => conns.push(Rc::new(ConnectionBackComplex1 {
                connection: self.connection(diamond2, diamond1),
                arrow_color: back_arrow_color,
            })),
        }

        let out_link_rendering = repeat.get_out_link_rendering();
        conns.push(Rc::new(ConnectionOut {
            connection: self.connection(repeat, diamond2),
            arrow_color: style.end_repeat_link_color.with_default(&style.arrow_color),
            tbout: style.arrow_text(
                out_link_rendering.display.as_ref(),
                skin_param,
                CreoleMode::SimpleLine,
            ),
        }));
        conns
    }

    fn connection(
        self: &Rc<Self>,
        ftile1: &Rc<dyn Ftile>,
        ftile2: &Rc<dyn Ftile>,
    ) -> RepeatConnection {
        RepeatConnection {
            base: AbstractConnection::new(Some(ftile1.clone()), Some(ftile2.clone())),
            tile: self.clone(),
        }
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_repeat = self.repeat.calculate_dimension(string_bounder);

        let w = self.tb_test.calculate_dimension(string_bounder).width;

        let mut width = self.get_left(string_bounder) + self.get_right(string_bounder);
        width = width.max(w + 2.0 * HEXAGON_HALF_SIZE);
        if let Some(backward) = &self.backward {
            width += backward.calculate_dimension(string_bounder).get_width();
        }

        let height = dim_diamond1.get_height()
            + dim_repeat.get_height()
            + dim_diamond2.get_height()
            + 8.0 * HEXAGON_HALF_SIZE;
        XDimension2D::new(width + 2.0 * HEXAGON_HALF_SIZE, height)
    }

    fn get_translate_for_repeat(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_repeat = self.repeat.calculate_dimension(string_bounder);
        let space = dim_total.height
            - dim_diamond1.get_height()
            - dim_diamond2.get_height()
            - dim_repeat.get_height();
        let y = dim_diamond1.get_height() + space / 2.0;
        let left = self.get_left(string_bounder);
        UTranslate::new(left - dim_repeat.get_left(), y)
    }

    fn get_translate_diamond1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let left = self.get_left(string_bounder);
        UTranslate::new(left - dim_diamond1.get_width() / 2.0, 0.0)
    }

    fn get_translate_backward(
        &self,
        backward: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_backward = backward.calculate_dimension(string_bounder);
        let x = dim_total.width - dim_backward.get_width();
        let y = (dim_total.height - dim_backward.get_height()) / 2.0;
        UTranslate::new(x, y)
    }

    fn get_translate_diamond2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let y2 = dim_total.height - dim_diamond2.get_height();
        let left = self.get_left(string_bounder);
        UTranslate::new(left - dim_diamond2.get_width() / 2.0, y2)
    }

    fn get_left(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let repeat_left = self.repeat.calculate_dimension(string_bounder).get_left();
        let left1 = repeat_left.max(dim_diamond1.get_width() / 2.0);
        let left2 = repeat_left.max(dim_diamond2.get_width() / 2.0);
        left1.max(left2)
    }

    fn get_right(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_repeat = self.repeat.calculate_dimension(string_bounder);
        let repeat_right = dim_repeat.get_width() - dim_repeat.get_left();
        let right1 = repeat_right.max(dim_diamond1.get_width() / 2.0);
        let right2 = repeat_right.max(dim_diamond2.get_width() / 2.0);
        right1.max(right2)
    }

    /// The top left corner of the entry diamond.
    fn get_diamond1_origin(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        self.get_translate_diamond1(string_bounder)
            .get_translated(XPoint2D::new(0.0, 0.0))
    }

    /// The top left corner of the test.
    fn get_diamond2_origin(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        self.get_translate_diamond2(string_bounder)
            .get_translated(XPoint2D::new(0.0, 0.0))
    }

    fn arrow(&self, color: &Rainbow, direction: Direction) -> Snake {
        Snake::create_with_end(
            self.skin_param(),
            color.clone(),
            self.skin_param().arrows().as_to(direction),
        )
    }
}

/// The test, drawn as `skinparam conditionStyle` says, with the labels of the arrows back (`yes`) and out
/// around it; also the test as written beside the loop, which only an empty diamond has. A loop ending
/// its parent without a test has an empty tile for a test (`placeholder`).
fn create_diamond2(
    style: &RepeatStyle,
    skin_param: &Rc<SkinParam>,
    (swimlane, swimlane_out): (Option<SwimlaneId>, Option<SwimlaneId>),
    [tb_test, yes_tb, out_tb]: [Rc<dyn TextBlock>; 3],
    placeholder: bool,
    yes_on_west: bool,
) -> (Rc<dyn Ftile>, Rc<dyn TextBlock>) {
    match style.condition_style {
        ConditionStyle::InsideHexagon => {
            let diamond2: Rc<dyn Ftile> = if placeholder {
                Rc::new(FtileEmpty::new(skin_param.clone(), None))
            } else {
                let diamond = FtileDiamondInside::new(
                    tb_test,
                    skin_param.clone(),
                    style.diamond_color2.clone(),
                    style.border_color.clone(),
                    swimlane_out,
                );
                let diamond = if yes_on_west {
                    diamond.with_west(yes_tb)
                } else {
                    diamond.with_east(yes_tb)
                };
                Rc::new(diamond.with_south(out_tb))
            };
            (diamond2, empty())
        }
        ConditionStyle::EmptyDiamond => (
            Rc::new(
                FtileDiamond::new(
                    skin_param.clone(),
                    style.diamond_color2.clone(),
                    style.border_color.clone(),
                    swimlane,
                )
                .with_east(tb_test.clone()),
            ),
            tb_test,
        ),
        ConditionStyle::InsideDiamond => (
            Rc::new(
                FtileDiamondSquare::new(
                    tb_test,
                    skin_param.clone(),
                    style.diamond_color2.clone(),
                    style.border_color.clone(),
                    swimlane,
                )
                .with_east(yes_tb)
                .with_south(out_tb),
            ),
            empty(),
        ),
    }
}

/// Whether the arrow back leaves the test on its left: towards a `backward` activity in a lane before the
/// test's, so the label of the arrow goes left of the test.
fn backward_exits_on_left(backward: Option<&dyn Ftile>, swimlane_out: Option<SwimlaneId>) -> bool {
    let swimlane_backward = backward.and_then(Swimable::get_swimlane_in);
    match (swimlane_backward, swimlane_out) {
        (Some(swimlane_backward), Some(swimlane_out)) => swimlane_backward < swimlane_out,
        _ => false,
    }
}

fn empty() -> Rc<dyn TextBlock> {
    Rc::new(TextBlockEmpty::default())
}

impl Swimable for FtileRepeat {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = self.repeat.get_swimlanes();
        result.extend(self.diamond1.get_swimlanes());
        result.extend(self.diamond2.get_swimlanes());
        if let Some(backward) = &self.backward {
            result.extend(backward.get_swimlanes());
        }
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.repeat.get_swimlane_in()
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.diamond2.get_swimlane_out()
    }
}

impl Ftile for FtileRepeat {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self.calculate_dimension_internal(string_bounder);
            FtileGeometry::from_dim_with_out(
                dim_total,
                self.get_left(string_bounder),
                0.0,
                dim_total.height,
            )
        })
    }

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.repeat.as_ref()) {
            return self.get_translate_for_repeat(string_bounder);
        }
        if same(child, self.diamond1.as_ref()) {
            return self.get_translate_diamond1(string_bounder);
        }
        UTranslate::default()
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![
            self.repeat.clone(),
            self.diamond1.clone(),
            self.diamond2.clone(),
        ]
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.apply(self.get_translate_for_repeat(string_bounder))
            .draw(&self.repeat);
        ug.apply(self.get_translate_diamond1(string_bounder))
            .draw(&self.diamond1);
        ug.apply(self.get_translate_diamond2(string_bounder))
            .draw(&self.diamond2);
        if let Some(backward) = &self.backward {
            ug.apply(self.get_translate_backward(backward.as_ref(), string_bounder))
                .draw(backward);
        }
    }
}

/// What the connections of a `repeat` share: the tiles they join and the loop they belong to (PlantUML's
/// inner classes reach the loop's fields).
struct RepeatConnection {
    base: AbstractConnection,
    tile: Rc<FtileRepeat>,
}

impl RepeatConnection {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        self.base.arrow_horizontal_alignment()
    }

    /// The top left corners of the test and of the entry.
    fn diamonds(&self, string_bounder: &dyn StringBounder) -> (XPoint2D, XPoint2D) {
        (
            self.tile.get_diamond2_origin(string_bounder),
            self.tile.get_diamond1_origin(string_bounder),
        )
    }

    /// The same, `translate1` moving the test's and `translate2` the entry's.
    fn diamonds_translated(
        &self,
        string_bounder: &dyn StringBounder,
        translate1: UTranslate,
        translate2: UTranslate,
    ) -> (XPoint2D, XPoint2D) {
        let (p1, p2) = self.diamonds(string_bounder);
        (translate1.get_translated(p1), translate2.get_translated(p2))
    }
}

/// From the entry down into the body.
struct ConnectionIn {
    connection: RepeatConnection,
    arrow_color: Rainbow,
    tbin: Rc<dyn TextBlock>,
}

impl ConnectionIn {
    fn get_p1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = &self.connection.tile;
        tile.diamond1
            .calculate_dimension(string_bounder)
            .translate(tile.get_translate_diamond1(string_bounder))
            .get_point_out()
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = &self.connection.tile;
        tile.repeat
            .calculate_dimension(string_bounder)
            .translate(tile.get_translate_for_repeat(string_bounder))
            .get_point_in()
    }

    fn draw_snake(&self, ug: &UGraphic, p1: XPoint2D, p2: XPoint2D) {
        let mut snake = self
            .connection
            .tile
            .arrow(&self.arrow_color, Direction::Down)
            .with_label(
                Some(self.tbin.clone()),
                self.connection.arrow_horizontal_alignment(),
            );
        snake.add_point_at(p1);
        if p1.x != p2.x {
            let my = f64::midpoint(p1.y, p2.y);
            snake.add_point(p1.x, my);
            snake.add_point(p2.x, my);
        }
        snake.add_point_at(p2);
        ug.draw(&snake);
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
        self.draw_snake(ug, self.get_p1(string_bounder), self.get_p2(string_bounder));
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionIn {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let p1 = translate1.get_translated(self.get_p1(string_bounder));
        let p2 = translate2.get_translated(self.get_p2(string_bounder));
        self.draw_snake(ug, p1, p2);
    }
}

/// From the end of the body down into the test.
struct ConnectionOut {
    connection: RepeatConnection,
    arrow_color: Rainbow,
    tbout: Rc<dyn TextBlock>,
}

impl ConnectionOut {
    fn get_p1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = &self.connection.tile;
        tile.get_translate_for_repeat(string_bounder)
            .get_translated(
                tile.repeat
                    .calculate_dimension(string_bounder)
                    .get_point_out(),
            )
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = &self.connection.tile;
        tile.get_translate_diamond2(string_bounder).get_translated(
            tile.diamond2
                .calculate_dimension(string_bounder)
                .get_point_in(),
        )
    }

    fn has_point_out(&self, string_bounder: &dyn StringBounder) -> bool {
        self.connection
            .tile
            .repeat
            .calculate_dimension(string_bounder)
            .has_point_out()
    }

    fn labelled_arrow(&self) -> Snake {
        self.connection
            .tile
            .arrow(&self.arrow_color, Direction::Down)
            .with_label(
                Some(self.tbout.clone()),
                self.connection.arrow_horizontal_alignment(),
            )
    }
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
        if !self.has_point_out(string_bounder) {
            return;
        }
        let mut snake = self.labelled_arrow();
        snake.add_point_at(self.get_p1(string_bounder));
        snake.add_point_at(self.get_p2(string_bounder));
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionOut {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        if !self.has_point_out(string_bounder) {
            return;
        }
        let mut snake = Snake::create(self.connection.tile.skin_param(), self.arrow_color.clone());
        let start = translate1.get_translated(self.get_p1(string_bounder));
        let end = translate2.get_translated(self.get_p2(string_bounder));
        let middle = f64::midpoint(start.y, end.y);
        snake.add_point_at(start);
        snake.add_point(start.x, middle);
        snake.add_point(end.x, middle);
        ug.draw(&snake);

        let mut small = self.labelled_arrow();
        small.add_point(end.x, middle);
        small.add_point_at(end);
        ug.draw(&small);
    }
}

/// From the test back to the entry when they lie in different lanes.
struct ConnectionBackComplex1 {
    connection: RepeatConnection,
    arrow_color: Rainbow,
}

impl ConnectionBackComplex1 {
    fn draw_snake(&self, ug: &UGraphic, p1: XPoint2D, p2: XPoint2D) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let dim_repeat = tile.repeat.calculate_dimension(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = tile.diamond2.calculate_dimension(string_bounder);
        let y1 = p1.y + dim_diamond2.get_height() / 2.0;
        let mut x2 = p2.x + dim_diamond1.get_width();
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        let x1_a = p1.x + dim_diamond2.get_width();
        let x1_b = p1.x
            + dim_diamond2.get_width() / 2.0
            + dim_repeat.get_width() / 2.0
            + HEXAGON_HALF_SIZE;

        let mut snake;
        if x2 < x1_a {
            snake = tile
                .arrow(&self.arrow_color, Direction::Left)
                .emphasize_direction(Direction::Up);
            snake.add_point(x1_a, y1);
            if x1_a < x1_b {
                snake.add_point(x1_b, y1);
                snake.add_point(x1_b, y2);
            } else {
                snake.add_point(x1_a + 10.0, y1);
                snake.add_point(x1_a + 10.0, y2);
            }
        } else {
            x2 = p2.x;
            snake = tile
                .arrow(&self.arrow_color, Direction::Right)
                .emphasize_direction(Direction::Up);
            snake.add_point(x1_a, y1);
            let middle = x1_a / 4.0 + x2 * 3.0 / 4.0;
            snake.add_point(middle, y1);
            snake.add_point(middle, y2);
        }
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }
}

impl Connection for ConnectionBackComplex1 {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let (p1, p2) = self.connection.diamonds(ug.string_bounder());
        self.draw_snake(ug, p1, p2);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionBackComplex1 {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let (p1, p2) =
            self.connection
                .diamonds_translated(ug.string_bounder(), translate1, translate2);
        self.draw_snake(ug, p1, p2);
    }
}

/// From the test, out the side the `backward` activity is on, up into it.
struct ConnectionBackBackward1 {
    connection: RepeatConnection,
    arrow_color: Rainbow,
    tbback: Rc<dyn TextBlock>,
}

impl ConnectionBackBackward1 {
    fn get_p2(&self, backward: &dyn Ftile, string_bounder: &dyn StringBounder) -> XPoint2D {
        let dim = backward.calculate_dimension(string_bounder);
        self.connection
            .tile
            .get_translate_backward(backward, string_bounder)
            .get_translated(XPoint2D::new(dim.get_left(), dim.get_out_y()))
    }

    fn draw_snake(&self, ug: &UGraphic, p1: XPoint2D, p2: XPoint2D) {
        let tile = &self.connection.tile;
        let dim_diamond2 = tile.diamond2.calculate_dimension(ug.string_bounder());
        let (x2, y2) = (p2.x, p2.y);
        // The arrow leaves the test on the side the backward activity is on.
        let diamond_center_x = p1.x + dim_diamond2.get_width() / 2.0;
        let x1 = if x2 < diamond_center_x {
            p1.x
        } else {
            p1.x + dim_diamond2.get_width()
        };
        let y1 = p1.y + dim_diamond2.get_height() / 2.0;

        let mut snake = tile.arrow(&self.arrow_color, Direction::Up).with_label(
            Some(self.tbback.clone()),
            self.connection.arrow_horizontal_alignment(),
        );
        snake.add_point(x1, y1);
        snake.add_point(x2, y1);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }
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
        let Some(backward) = &tile.backward else {
            return;
        };
        let p1 = tile.get_diamond2_origin(string_bounder);
        let p2 = self.get_p2(backward.as_ref(), string_bounder);
        self.draw_snake(ug, p1, p2);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionBackBackward1 {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let Some(backward) = &tile.backward else {
            return;
        };
        let p1 = translate1.get_translated(tile.get_diamond2_origin(string_bounder));
        let p2 = translate2.get_translated(self.get_p2(backward.as_ref(), string_bounder));
        self.draw_snake(ug, p1, p2);
    }
}

/// From the `backward` activity up and into the entry.
struct ConnectionBackBackward2 {
    connection: RepeatConnection,
    arrow_color: Rainbow,
    label: Rc<dyn TextBlock>,
}

impl ConnectionBackBackward2 {
    fn get_p1(&self, backward: &dyn Ftile, string_bounder: &dyn StringBounder) -> XPoint2D {
        let dim = backward.calculate_dimension(string_bounder);
        self.connection
            .tile
            .get_translate_backward(backward, string_bounder)
            .get_translated(XPoint2D::new(dim.get_left(), dim.get_in_y()))
    }

    fn labelled(&self, snake: Snake) -> Snake {
        snake.with_label(
            Some(self.label.clone()),
            self.connection.arrow_horizontal_alignment(),
        )
    }
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
        let mut snake = self.labelled(tile.arrow(&self.arrow_color, Direction::Left));
        let p1 = self.get_p1(backward.as_ref(), string_bounder);
        let p2 = tile.get_diamond1_origin(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);
        let (x1, y1) = (p1.x, p1.y);
        let x2 = p2.x + dim_diamond1.get_width();
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        snake.add_point(x1, y1);
        snake.add_point(x1, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionBackBackward2 {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let Some(backward) = &tile.backward else {
            return;
        };
        let p1 = translate1.get_translated(self.get_p1(backward.as_ref(), string_bounder));
        let p2 = translate2.get_translated(tile.get_diamond1_origin(string_bounder));
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);

        let (x1, y1) = (p1.x, p1.y);
        let mut x2 = p2.x;
        if x2 < x1 {
            x2 += dim_diamond1.get_width();
        }
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        let direction = if x2 < x1 {
            Direction::Left
        } else {
            Direction::Right
        };
        let mut snake = self.labelled(tile.arrow(&self.arrow_color, direction));
        snake.add_point(x1, y1);
        snake.add_point(x1, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }
}

/// From the test, around the left of the body, back into the entry: the loop's lane is left of all the
/// body's.
struct ConnectionBackSimple1 {
    connection: RepeatConnection,
    arrow_color: Rainbow,
    tbback: Rc<dyn TextBlock>,
}

impl ConnectionBackSimple1 {
    fn arrow(&self, direction: Direction) -> Snake {
        self.connection
            .tile
            .arrow(&self.arrow_color, direction)
            .emphasize_direction(Direction::Up)
            .with_label(
                Some(self.tbback.clone()),
                self.connection.arrow_horizontal_alignment(),
            )
    }
}

impl Connection for ConnectionBackSimple1 {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let mut snake = self.arrow(Direction::Right);
        let (p1, p2) = self.connection.diamonds(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = tile.diamond2.calculate_dimension(string_bounder);
        let x1 = p1.x;
        let y1 = p1.y + dim_diamond2.get_height() / 2.0;
        let x2 = p2.x;
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        snake.add_point(x1, y1);
        let xmin = -HEXAGON_HALF_SIZE;
        snake.add_point(xmin, y1);
        snake.add_point(xmin, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionBackSimple1 {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let mut snake = self.arrow(Direction::Left);
        let dim_repeat = tile.repeat.calculate_dimension(string_bounder);
        let (p1, p2) = self
            .connection
            .diamonds_translated(string_bounder, translate1, translate2);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = tile.diamond2.calculate_dimension(string_bounder);
        let x1 = p1.x;
        let y1 = p1.y + dim_diamond2.get_height() / 2.0;
        let x2 = p2.x;
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        snake.add_point(x1, y1);
        let xmax = p1.x
            + dim_diamond2.get_width() / 2.0
            + dim_repeat.get_width() / 2.0
            + HEXAGON_HALF_SIZE;
        snake.add_point(xmax, y1);
        snake.add_point(xmax, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }
}

/// From the test, around the right of the body, back into the entry.
struct ConnectionBackSimple2 {
    connection: RepeatConnection,
    arrow_color: Rainbow,
    tbback: Rc<dyn TextBlock>,
}

impl ConnectionBackSimple2 {
    fn arrow(&self, direction: Direction) -> Snake {
        self.connection
            .tile
            .arrow(&self.arrow_color, direction)
            .emphasize_direction(Direction::Up)
            .with_label(
                Some(self.tbback.clone()),
                self.connection.arrow_horizontal_alignment(),
            )
    }
}

impl Connection for ConnectionBackSimple2 {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.connection.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let mut snake = self.arrow(Direction::Left);
        let dim_total = tile.calculate_dimension_internal(string_bounder);
        let (p1, p2) = self.connection.diamonds(string_bounder);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = tile.diamond2.calculate_dimension(string_bounder);
        let x1 = p1.x + dim_diamond2.get_width();
        let y1 = p1.y + dim_diamond2.get_height() / 2.0;
        let x2 = p2.x + dim_diamond1.get_width();
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        snake.add_point(x1, y1);
        let xmax = dim_total.width - HEXAGON_HALF_SIZE;
        snake.add_point(xmax, y1);
        snake.add_point(xmax, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionBackSimple2 {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let tile = &self.connection.tile;
        let (p1, p2) = self
            .connection
            .diamonds_translated(string_bounder, translate1, translate2);
        let dim_diamond1 = tile.diamond1.calculate_dimension(string_bounder);
        let dim_diamond2 = tile.diamond2.calculate_dimension(string_bounder);
        let x1 = p1.x + dim_diamond2.get_width();
        let y1 = p1.y + dim_diamond2.get_height() / 2.0;

        let x2a = p2.x;
        let x2b = p2.x + dim_diamond1.get_width();
        let is_on_a = x1 < f64::midpoint(x2a, x2b);

        let x2 = if is_on_a { x2a } else { x2b };
        let y2 = p2.y + dim_diamond1.get_height() / 2.0;

        let mut snake = self.arrow(if is_on_a {
            Direction::Right
        } else {
            Direction::Left
        });
        snake.add_point(x1, y1);
        let xmiddle = f64::midpoint(x1, x2);
        snake.add_point(xmiddle, y1);
        snake.add_point(xmiddle, y2);
        snake.add_point(x2, y2);
        ug.draw(&snake);
    }
}
