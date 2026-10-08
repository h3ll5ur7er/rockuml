//! An `if` with `elseif`s: the conditions side by side, each above its branch, the `else` last (PlantUML's
//! `FtileIfLongHorizontal`).

use std::rc::Rc;

use super::create0_or_empty;
use super::one_swimlane::hline_extent;
use crate::color::HColor;
use crate::decoration::Rainbow;
use crate::diagram::activity3::{BranchFtile, LinkRendering, SwimlaneId, SwimlaneSet};
use crate::ftile::vertical::FtileDiamondInside2;
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, ConnectionTranslatable, Ftile,
    FtileAssemblySimple, FtileFactory, FtileGeometry, FtileMinWidthCentered, MergeStrategy, Snake,
    Swimable, ftile_utils, same,
};
use crate::creole::Display;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::creole::CreoleMode;
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};

const X_SEPARATION: f64 = 20.0;

pub(crate) struct FtileIfLongHorizontal {
    base: AbstractFtile,
    tiles: Vec<Rc<dyn Ftile>>,
    tile2: Rc<dyn Ftile>,
    diamonds: Vec<Rc<dyn Ftile>>,
    /// Each diamond above its branch, with room left for the label of the arrow into the diamond.
    couples: Vec<Rc<dyn Ftile>>,
    arrow_color: Rainbow,
}

impl FtileIfLongHorizontal {
    /// The conditional of `thens`, ending with `branch2`, its `else`.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's create")]
    pub(crate) fn create(
        swimlane: Option<SwimlaneId>,
        back_color: &HColor,
        ftile_factory: &dyn FtileFactory,
        thens: &[BranchFtile<'_>],
        branch2: &BranchFtile<'_>,
        top_inlink_rendering: &LinkRendering,
        after_endwhile: &LinkRendering,
        style_arrow: &Style,
        style_diamond: &Style,
    ) -> Rc<dyn Ftile> {
        let skin_param = ftile_factory.skin_param();
        let string_bounder = ftile_factory.get_string_bounder();
        let border_color = style_diamond.value(PName::LineColor).as_color();
        let arrow_color = Rainbow::build_from_style(style_arrow);
        let fc_test = style_diamond.font_configuration();
        let fc_arrow = style_arrow.font_configuration();
        let create = |display: Option<&Display>, font: &FontConfiguration| {
            create0_or_empty(
                display,
                font,
                HorizontalAlignment::Left,
                skin_param,
                0.0,
                CreoleMode::Full,
            )
        };
        let tiles: Vec<Rc<dyn Ftile>> = thens
            .iter()
            .map(|branch| min_width_centered(&branch.ftile))
            .collect();
        let tile2 = min_width_centered(&branch2.ftile);
        let mut diamonds = Vec::new();
        let mut inlabel_sizes = Vec::new();
        for branch in thens {
            let tb1 = create(branch.get_display_positive().as_ref(), &fc_arrow);
            let tb_test = create0_or_empty(
                branch.branch.label_test.as_ref(),
                &fc_test,
                skin_param.get_default_text_alignment(HorizontalAlignment::Left),
                skin_param,
                style_diamond.wrap_width(),
                CreoleMode::Full,
            );
            let diamond_color = branch
                .branch
                .get_color()
                .unwrap_or_else(|| back_color.clone());
            let mut diamond = FtileDiamondInside2::new(
                tb_test,
                Rc::clone(skin_param),
                diamond_color,
                border_color.clone(),
                swimlane,
            );
            match branch.branch.get_inlabel() {
                None => inlabel_sizes.push(0.0),
                Some(inlabel) => {
                    let tb_inlabel = create(Some(inlabel), &fc_arrow);
                    inlabel_sizes.push(tb_inlabel.calculate_dimension(string_bounder).width);
                    diamond = diamond.with_west(tb_inlabel);
                }
            }
            diamonds.push(diamond.with_north(tb1));
        }
        let tb2 = create(branch2.get_display_positive().as_ref(), &fc_arrow);
        let mut diamonds: Vec<Rc<dyn Ftile>> = match diamonds.pop() {
            Some(last) => diamonds
                .into_iter()
                .chain([last.with_east(tb2)])
                .map(|diamond| Rc::new(diamond) as Rc<dyn Ftile>)
                .collect(),
            None => Vec::new(),
        };
        diamonds = align_diamonds(&diamonds, string_bounder);
        let couples = diamonds
            .iter()
            .zip(&tiles)
            .zip(&inlabel_sizes)
            .map(|((diamond, tile), inlabel_size)| {
                let tmp: Rc<dyn Ftile> =
                    Rc::new(FtileAssemblySimple::new(Rc::clone(diamond), Rc::clone(tile)));
                ftile_utils::add_horizontal_margin(tmp, *inlabel_size, 0.0)
            })
            .collect();
        let result = Rc::new(Self {
            base: AbstractFtile::new(Rc::clone(skin_param)),
            tiles: tiles.clone(),
            tile2: Rc::clone(&tile2),
            diamonds: diamonds.clone(),
            couples,
            arrow_color: arrow_color.clone(),
        });
        let or_arrow_color = |color: Rainbow| {
            if color.size() == 0 {
                arrow_color.clone()
            } else {
                color
            }
        };
        let special_label = |branch: &BranchFtile<'_>| {
            branch
                .branch
                .special
                .as_ref()
                .map(|special| create(special.display.as_ref(), &fc_test))
        };
        let mut conns: Vec<Rc<dyn Connection>> = Vec::new();
        let mut nb_out = 0;
        for ((tile, diamond), branch) in tiles.iter().zip(&diamonds).zip(thens) {
            let rainbow_in = branch.get_in_color(&arrow_color);
            if branch
                .ftile
                .calculate_dimension(string_bounder)
                .has_point_out()
            {
                nb_out += 1;
            }
            let rainbow_out = branch.branch.get_out();
            conns.push(Rc::new(ConnectionVerticalIn {
                parent: Rc::clone(&result),
                base: AbstractConnection::new(Some(Rc::clone(diamond)), Some(Rc::clone(tile))),
                color: or_arrow_color(rainbow_in),
            }));
            conns.push(Rc::new(ConnectionVerticalOut {
                parent: Rc::clone(&result),
                base: AbstractConnection::new(Some(Rc::clone(tile)), None),
                color: or_arrow_color(rainbow_out),
                out2: special_label(branch),
            }));
        }
        let top_in_color = top_inlink_rendering.get_rainbow_or(&arrow_color);
        for (pair, branch) in diamonds.windows(2).zip(thens.iter().skip(1)) {
            conns.push(Rc::new(ConnectionHorizontal {
                parent: Rc::clone(&result),
                base: AbstractConnection::new(Some(Rc::clone(&pair[0])), Some(Rc::clone(&pair[1]))),
                color: branch.branch.get_in_rainbow(&arrow_color),
            }));
        }
        conns.push(Rc::new(ConnectionIn {
            parent: Rc::clone(&result),
            base: AbstractConnection::new(None, diamonds.first().cloned()),
            arrow_color: top_in_color,
        }));
        let rainbow_out = branch2.branch.get_out();
        let rainbow_in = branch2.get_in_color(&arrow_color);
        conns.push(Rc::new(ConnectionLastElseIn {
            parent: Rc::clone(&result),
            base: AbstractConnection::new(diamonds.last().cloned(), Some(Rc::clone(&tile2))),
            arrow_color: or_arrow_color(rainbow_in),
        }));
        conns.push(Rc::new(ConnectionLastElseOut {
            parent: Rc::clone(&result),
            base: AbstractConnection::new(Some(Rc::clone(&tile2)), None),
            arrow_color: or_arrow_color(rainbow_out),
            out2: special_label(branch2),
            nb_out,
        }));
        if nb_out > 0 {
            conns.push(Rc::new(ConnectionHline {
                parent: Rc::clone(&result),
                base: AbstractConnection::new(None, None),
                arrow_color: after_endwhile.get_rainbow_or(&arrow_color),
            }));
        }
        ftile_utils::add_connections(result, conns)
    }

    fn get_translate2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_width() - dim2.get_width(),
            (dim_total.get_height() - dim2.get_height()) / 2.0,
        )
    }

    /// Where `diamond` is drawn, through the couple holding it.
    fn get_translate_diamond1(&self, diamond: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        self.through_couple(&self.diamonds, diamond, string_bounder)
    }

    /// Where `tile`, a branch, is drawn, through the couple holding it.
    fn get_translate1(&self, tile: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        self.through_couple(&self.tiles, tile, string_bounder)
    }

    fn through_couple(
        &self,
        list: &[Rc<dyn Ftile>],
        tile: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let Some(idx) = list.iter().position(|other| same(other.as_ref(), tile)) else {
            return UTranslate::default();
        };
        let couple = &self.couples[idx];
        let tr_couple = self.get_translate_couple1(couple.as_ref(), string_bounder);
        tr_couple.compose(couple.get_translate_for(tile, string_bounder))
    }

    fn get_translate_couple1(&self, candidate: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        let mut x1 = 0.0;
        for couple in &self.couples {
            if same(couple.as_ref(), candidate) {
                return UTranslate::new(x1, 25.0);
            }
            x1 += couple.calculate_dimension(string_bounder).get_width() + X_SEPARATION;
        }
        UTranslate::default()
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let max_out_y = get_max_out_y(&self.diamonds, string_bounder);
        let mut result = XDimension2D::new(0.0, 0.0);
        for couple in &self.couples {
            result = result.merge_lr(couple.calculate_dimension(string_bounder).dimension());
        }
        let dim_tile2 = self
            .tile2
            .calculate_dimension(string_bounder)
            .dimension()
            .delta(0.0, self.get_diamonds_height(string_bounder) / 2.0);
        let result = result
            .merge_lr(dim_tile2)
            .delta(X_SEPARATION * self.couples.len() as f64, max_out_y.max(100.0));
        FtileGeometry::from_dim(result, result.width / 2.0, 0.0)
    }

    fn get_diamonds_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.diamonds.iter().fold(0.0, |height: f64, diamond| {
            height.max(diamond.calculate_dimension(string_bounder).get_height())
        })
    }

    fn get_ydiamont_out_to_left(dim_diamond1: FtileGeometry) -> f64 {
        f64::midpoint(dim_diamond1.get_in_y(), dim_diamond1.get_out_y())
    }

    /// The right point of `diamond`, where the arrow to the next condition leaves.
    fn get_east_of(&self, diamond: &dyn Ftile, string_bounder: &dyn StringBounder) -> XPoint2D {
        let dim_diamond = diamond.calculate_dimension(string_bounder);
        let p = XPoint2D::new(
            dim_diamond.get_left() * 2.0,
            Self::get_ydiamont_out_to_left(dim_diamond),
        );
        self.get_translate_diamond1(diamond, string_bounder)
            .get_translated(p)
    }
}

fn min_width_centered(tile: &Rc<dyn Ftile>) -> Rc<dyn Ftile> {
    Rc::new(FtileMinWidthCentered::new(Rc::clone(tile), 30.0))
}

/// The diamonds with room above, so that arrows leave them all at the same height, and 20 below.
fn align_diamonds(diamonds: &[Rc<dyn Ftile>], string_bounder: &dyn StringBounder) -> Vec<Rc<dyn Ftile>> {
    let max_out_y = get_max_out_y(diamonds, string_bounder);
    diamonds
        .iter()
        .map(|diamond| {
            let missing = max_out_y - diamond.calculate_dimension(string_bounder).get_out_y();
            ftile_utils::add_vertical_margin(Rc::clone(diamond), missing / 2.0, 20.0)
        })
        .collect()
}

fn get_max_out_y(diamonds: &[Rc<dyn Ftile>], string_bounder: &dyn StringBounder) -> f64 {
    diamonds.iter().fold(0.0, |max_out_y: f64, diamond| {
        max_out_y.max(diamond.calculate_dimension(string_bounder).get_out_y())
    })
}

impl Swimable for FtileIfLongHorizontal {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = SwimlaneSet::new();
        if let Some(in_) = self.get_swimlane_in() {
            result.insert(Some(in_));
        }
        for couple in &self.couples {
            result.extend(couple.get_swimlanes());
        }
        result.extend(self.tile2.get_swimlanes());
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.couples.first().and_then(|couple| couple.get_swimlane_in())
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.get_swimlane_in()
    }
}

impl Ftile for FtileIfLongHorizontal {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self.calculate_dimension_internal(string_bounder).dimension();
            let any_out = self
                .tiles
                .iter()
                .chain([&self.tile2])
                .any(|tile| tile.calculate_dimension(string_bounder).has_point_out());
            if any_out {
                FtileGeometry::from_dim_with_out(
                    dim_total,
                    dim_total.width / 2.0,
                    0.0,
                    dim_total.height,
                )
            } else {
                FtileGeometry::from_dim(dim_total, dim_total.width / 2.0, 0.0)
            }
        })
    }

    fn get_translate_for(&self, child: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        if same(child, self.tile2.as_ref()) {
            return self.get_translate2(string_bounder);
        }
        if self.couples.iter().any(|couple| same(couple.as_ref(), child)) {
            return self.get_translate_couple1(child, string_bounder);
        }
        self.get_translate1(child, string_bounder)
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.tiles
            .iter()
            .chain([&self.tile2])
            .cloned()
            .collect()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        for couple in &self.couples {
            ug.apply(self.get_translate_couple1(couple.as_ref(), string_bounder))
                .draw(couple);
        }
        ug.apply(self.get_translate2(string_bounder))
            .draw(&self.tile2);
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

/// From a condition to the next.
struct ConnectionHorizontal {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    color: Rainbow,
}

impl Connection for ConnectionHorizontal {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let (Some(diamond1), Some(diamond2)) = (self.base.get_ftile1(), self.base.get_ftile2()) else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let p1 = parent.get_east_of(diamond1.as_ref(), string_bounder);
        let dim_diamond2 = diamond2.calculate_dimension(string_bounder);
        let p2 = parent
            .get_translate_diamond1(diamond2.as_ref(), string_bounder)
            .get_translated(XPoint2D::new(
                0.0,
                FtileIfLongHorizontal::get_ydiamont_out_to_left(dim_diamond2),
            ));
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(skin_param, self.color.clone(), skin_param.arrows().as_to_right());
        snake.add_point_at(p1);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }
}

/// Into the first condition.
struct ConnectionIn {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    arrow_color: Rainbow,
}

impl Connection for ConnectionIn {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let Some(diamond) = self.base.get_ftile2() else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let p2 = parent
            .get_translate_diamond1(diamond.as_ref(), string_bounder)
            .get_translated(diamond.calculate_dimension(string_bounder).get_point_in());
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(skin_param, self.arrow_color.clone(), skin_param.arrows().as_to_down());
        let p1 = parent
            .calculate_dimension_internal(string_bounder)
            .get_point_in();
        snake.add_point_at(p1);
        snake.add_point(p2.x, p1.y);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }
}

/// From the last condition into the `else`.
struct ConnectionLastElseIn {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    arrow_color: Rainbow,
}

impl Connection for ConnectionLastElseIn {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let Some(diamond) = self.base.get_ftile1() else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let p1 = parent.get_east_of(diamond.as_ref(), string_bounder);
        let p2 = parent
            .get_translate2(string_bounder)
            .get_translated(parent.tile2.calculate_dimension(string_bounder).get_point_in());
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(skin_param, self.arrow_color.clone(), skin_param.arrows().as_to_down());
        snake.add_point_at(p1);
        snake.add_point(p2.x, p1.y);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }
}

/// From the `else` down to the bottom, and along it when no other branch goes on.
struct ConnectionLastElseOut {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    arrow_color: Rainbow,
    out2: Option<Rc<dyn TextBlock>>,
    nb_out: usize,
}

impl Connection for ConnectionLastElseOut {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let dim = parent.tile2.calculate_dimension(string_bounder);
        if !dim.has_point_out() {
            return;
        }
        let p1 = parent
            .get_translate2(string_bounder)
            .get_translated(dim.get_point_out());
        let full = parent.calculate_dimension_internal(string_bounder);
        let total_height = full.get_height();
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(skin_param, self.arrow_color.clone(), skin_param.arrows().as_to_down())
            .with_label(self.out2.clone(), self.base.arrow_horizontal_alignment());
        snake.add_point_at(p1);
        snake.add_point(p1.x, total_height);
        if self.nb_out == 0 {
            snake.add_point(full.get_left(), total_height);
        }
        ug.draw(&snake);
    }
}

/// From a condition down into its branch.
struct ConnectionVerticalIn {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    color: Rainbow,
}

impl ConnectionVerticalIn {
    fn points(&self, string_bounder: &dyn StringBounder) -> Option<(XPoint2D, XPoint2D)> {
        let (diamond, tile) = (self.base.get_ftile1()?, self.base.get_ftile2()?);
        let parent = &self.parent;
        let p1 = parent
            .get_translate_diamond1(diamond.as_ref(), string_bounder)
            .get_translated(diamond.calculate_dimension(string_bounder).get_point_out());
        let p2 = parent
            .get_translate1(tile.as_ref(), string_bounder)
            .get_translated(tile.calculate_dimension(string_bounder).get_point_in());
        Some((p1, p2))
    }

    fn snake(&self) -> Snake {
        let skin_param = self.parent.skin_param();
        Snake::create_with_end(skin_param, self.color.clone(), skin_param.arrows().as_to_down())
    }
}

impl Connection for ConnectionVerticalIn {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let Some((p1, p2)) = self.points(ug.string_bounder()) else {
            return;
        };
        let mut snake = self.snake();
        snake.add_point_at(p1);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionVerticalIn {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let Some((p1, p2)) = self.points(ug.string_bounder()) else {
            return;
        };
        let mp1a = translate1.get_translated(p1);
        let mp2b = translate2.get_translated(p2);
        let middle = mp1a.y + 4.0;
        let mut snake = self.snake();
        snake.add_point_at(mp1a);
        snake.add_point(mp1a.x, middle);
        snake.add_point(mp2b.x, middle);
        snake.add_point_at(mp2b);
        ug.draw(&snake);
    }
}

/// From a branch down to the bottom.
struct ConnectionVerticalOut {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    color: Rainbow,
    out2: Option<Rc<dyn TextBlock>>,
}

impl Connection for ConnectionVerticalOut {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let Some(tile) = self.base.get_ftile1() else {
            return;
        };
        let string_bounder = ug.string_bounder();
        let parent = &self.parent;
        let total_height = parent
            .calculate_dimension_internal(string_bounder)
            .get_height();
        let geo = tile.calculate_dimension(string_bounder);
        if !geo.has_point_out() {
            return;
        }
        let p1 = parent
            .get_translate1(tile.as_ref(), string_bounder)
            .get_translated(geo.get_point_out());
        let skin_param = parent.skin_param();
        let mut snake = Snake::create_with_end(skin_param, self.color.clone(), skin_param.arrows().as_to_down())
            .with_label(self.out2.clone(), self.base.arrow_horizontal_alignment());
        snake.add_point_at(p1);
        snake.add_point(p1.x, total_height);
        ug.draw(&snake);
    }
}

/// The line at the bottom joining the branches that go on.
struct ConnectionHline {
    parent: Rc<FtileIfLongHorizontal>,
    base: AbstractConnection,
    arrow_color: Rainbow,
}

impl Connection for ConnectionHline {
    connection_tiles!();

    fn draw_u(&self, ug: &UGraphic) {
        let parent = &self.parent;
        let string_bounder = ug.string_bounder();
        let total_dim = parent.calculate_dimension_internal(string_bounder);
        let all_tiles: Vec<Rc<dyn Ftile>> = parent
            .couples
            .iter()
            .chain([&parent.tile2])
            .cloned()
            .collect();
        let dim = parent.calculate_dimension(string_bounder);
        if !dim.has_point_out() {
            return;
        }
        let Some((min_x, max_x)) = hline_extent(
            ug,
            total_dim.get_width(),
            &all_tiles,
            Some(dim.get_left()),
            |tile, string_bounder| parent.get_translate_for(tile, string_bounder),
        ) else {
            return;
        };
        let mut snake = Snake::create(parent.skin_param(), self.arrow_color.clone())
            .with_merge(MergeStrategy::None);
        snake.add_point(min_x, total_dim.get_height());
        snake.add_point(max_x, total_dim.get_height());
        ug.draw(&snake);
    }
}
