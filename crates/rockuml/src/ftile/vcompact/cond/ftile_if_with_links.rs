//! An `if` with an `else`, both branches side by side between two diamonds (PlantUML's `FtileIfWithLinks`,
//! with its bases `FtileIfWithDiamonds` and `FtileIfNude`, which nothing else builds).
//!
//! Notes on the diamond (`FtileIfWithDiamonds`' opales) are not ported: their offsets are 0.

use std::cell::OnceCell;
use std::rc::Rc;

use crate::decoration::Rainbow;
use crate::diagram::activity3::{BranchFtile, SwimlaneId, SwimlaneSet};
use crate::direction::Direction;
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::vcompact::one_swimlane::hline_extent;
use crate::ftile::vertical::FtileDiamond;
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, ConnectionTranslatable, Ftile, FtileGeometry,
    MergeStrategy, Snake, Swimable, downcast, ftile_utils, same,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::shape::UPolygon;
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::svek::ConditionEndStyle;

/// Room beside the first diamond.
const SUPP_WIDTH: f64 = 20.0;

pub(crate) struct FtileIfWithLinks {
    base: AbstractFtile,
    /// `FtileDimensionMemoize`: the geometry with a point out whatever the branches do.
    dimension_internal: OnceCell<FtileGeometry>,
    tile1: Rc<dyn Ftile>,
    tile2: Rc<dyn Ftile>,
    diamond1: Rc<dyn Ftile>,
    diamond2: Rc<dyn Ftile>,
    in_: Option<SwimlaneId>,
    arrow_color: Rainbow,
    condition_end_style: ConditionEndStyle,
}

impl FtileIfWithLinks {
    #[allow(clippy::too_many_arguments, reason = "PlantUML's constructor")]
    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        diamond1: Rc<dyn Ftile>,
        tile1: Rc<dyn Ftile>,
        tile2: Rc<dyn Ftile>,
        diamond2: Rc<dyn Ftile>,
        in_: Option<SwimlaneId>,
        arrow_color: Rainbow,
        condition_end_style: ConditionEndStyle,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            dimension_internal: OnceCell::new(),
            tile1,
            tile2,
            diamond1,
            diamond2,
            in_,
            arrow_color,
            condition_end_style,
        }
    }

    fn has_two_branches(&self, string_bounder: &dyn StringBounder) -> bool {
        self.tile1
            .calculate_dimension(string_bounder)
            .has_point_out()
            && self
                .tile2
                .calculate_dimension(string_bounder)
                .has_point_out()
    }

    fn get_ydelta1a(&self) -> f64 {
        if self.get_swimlanes().len() > 1 {
            20.0
        } else {
            10.0
        }
    }

    fn get_ydelta1b(&self, string_bounder: &dyn StringBounder) -> f64 {
        if self.get_swimlanes().len() > 1 {
            10.0
        } else if self.has_two_branches(string_bounder) {
            6.0
        } else {
            0.0
        }
    }

    fn get_ydelta_for_labels(&self, string_bounder: &dyn StringBounder) -> f64 {
        downcast::<FtileDiamond>(self.diamond2.as_ref())
            .map_or(0.0, |diamond2| {
                diamond2.get_west_east_label_height(string_bounder)
            })
    }

    /// The room between the branches (`FtileIfWithDiamonds.widthInner`).
    fn width_inner(&self, string_bounder: &dyn StringBounder) -> f64 {
        let dim1 = self.tile1.calculate_dimension(string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        let nude = (dim1.get_width() - dim1.get_left()) + dim2.get_left();
        let diamond1 = self.diamond1.calculate_dimension(string_bounder);
        nude.max(diamond1.get_width() + SUPP_WIDTH)
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        *self
            .dimension_internal
            .get_or_init(|| self.calculate_dimension_internal_slow(string_bounder))
    }

    fn calculate_dimension_internal_slow(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let dim1 = self.diamond1.calculate_dimension(string_bounder);
        let dim2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_nude = self.calculate_dimension_nude(string_bounder);
        let all = dim1.append_bottom(dim_nude).append_bottom(dim2);
        let delta_height = self.get_ydelta1a()
            + self.get_ydelta1b(string_bounder)
            + self.get_ydelta_for_labels(string_bounder);
        all.add_dim(0.0, delta_height)
    }

    /// The branches side by side (`FtileIfNude.calculateDimensionInternalSlow`).
    fn calculate_dimension_nude(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let dim1 = self.tile1.calculate_dimension(string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        let inner_margin = self.width_inner(string_bounder);
        let width = dim1.get_left() + inner_margin + (dim2.get_width() - dim2.get_left());
        let height = dim1.dimension().merge_lr(dim2.dimension()).height;
        FtileGeometry::with_out(width, height, dim1.get_left() + inner_margin / 2.0, 0.0, height)
    }

    fn get_translate_branch1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(0.0, dim_diamond1.get_height() + self.get_ydelta1a())
    }

    fn get_translate_branch2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_width() - dim2.get_width(),
            dim_diamond1.get_height() + self.get_ydelta1a(),
        )
    }

    fn get_translate_diamond1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(dim_total.get_left() - dim_diamond1.get_left(), 0.0)
    }

    fn get_translate_diamond2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_left() - dim_diamond2.get_width() / 2.0,
            dim_total.get_height() - dim_diamond2.get_height(),
        )
    }

    /// Where `tile`, a branch, is drawn (the connections' `translate`); PlantUML fails on another tile.
    fn translate_branch(&self, tile: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        if same(tile, self.tile2.as_ref()) {
            self.get_translate_branch2(string_bounder)
        } else {
            self.get_translate_branch1(string_bounder)
        }
    }

    /// How much room the label of the first branch needs left of the tile (`computeMarginNeedForBranchLabe1`).
    pub(crate) fn compute_margin_need_for_branch_labe1(
        &self,
        string_bounder: &dyn StringBounder,
        label1: XDimension2D,
    ) -> f64 {
        let diff = label1.width - self.get_translate_diamond1(string_bounder).dx;
        if diff > 0.0 { diff } else { 0.0 }
    }

    /// How much room the label of the second branch needs right of the tile
    /// (`computeMarginNeedForBranchLabe2`).
    pub(crate) fn compute_margin_need_for_branch_labe2(
        &self,
        string_bounder: &dyn StringBounder,
        label2: XDimension2D,
    ) -> f64 {
        let theorical_end_needed = self.get_translate_diamond1(string_bounder).dx
            + self
                .diamond1
                .calculate_dimension(string_bounder)
                .get_width()
            + label2.width;
        let diff = theorical_end_needed - self.calculate_dimension(string_bounder).get_width();
        if diff > 0.0 { diff } else { 0.0 }
    }

    /// How much room the labels of the branches need below the diamond
    /// (`computeVerticalMarginNeedForBranchs`).
    pub(crate) fn compute_vertical_margin_need_for_branchs(
        &self,
        string_bounder: &dyn StringBounder,
        label1: XDimension2D,
        label2: XDimension2D,
    ) -> f64 {
        let height_labels = label1.height.max(label2.height);
        let dy_diamond = self
            .diamond1
            .calculate_dimension(string_bounder)
            .get_height();
        let diff = height_labels - dy_diamond;
        if diff > 0.0 { diff } else { 0.0 }
    }

    /// The tile with its arrows (`addLinks`).
    pub(crate) fn add_links(
        self: Rc<Self>,
        branch1: &BranchFtile<'_>,
        branch2: &BranchFtile<'_>,
        string_bounder: &dyn StringBounder,
    ) -> Rc<dyn Ftile> {
        let tile1 = Rc::clone(&self.tile1);
        let tile2 = Rc::clone(&self.tile2);
        let mut conns: Vec<Rc<dyn Connection>> = vec![
            Rc::new(ConnectionHorizontalThenVertical::new(&self, &tile1, branch1)),
            Rc::new(ConnectionHorizontalThenVertical::new(&self, &tile2, branch2)),
        ];
        let has_point_out1 = tile1.calculate_dimension(string_bounder).has_point_out();
        let has_point_out2 = tile2.calculate_dimension(string_bounder).has_point_out();
        let direct = |tile: &Rc<dyn Ftile>, branch: &BranchFtile<'_>| -> Rc<dyn Connection> {
            Rc::new(ConnectionVerticalThenHorizontalDirect::new(
                &self,
                tile,
                &branch.branch.get_out(),
                branch.is_empty(),
            ))
        };
        match (self.condition_end_style, has_point_out1, has_point_out2) {
            (ConditionEndStyle::Diamond, true, true) => {
                conns.push(Rc::new(ConnectionVerticalThenHorizontal::new(
                    &self,
                    &tile1,
                    &branch1.branch.get_out(),
                    branch1.is_empty(),
                )));
                conns.push(Rc::new(ConnectionVerticalThenHorizontal::new(
                    &self,
                    &tile2,
                    &branch2.branch.get_out(),
                    branch2.is_empty(),
                )));
            }
            (ConditionEndStyle::Hline, true, true) => {
                conns.push(Rc::new(ConnectionVerticalOut::new(&self, &tile1)));
                conns.push(Rc::new(ConnectionVerticalOut::new(&self, &tile2)));
                conns.push(Rc::new(ConnectionHline {
                    parent: Rc::clone(&self),
                    base: AbstractConnection::new(None, None),
                }));
            }
            (_, true, false) => conns.push(direct(&tile1, branch1)),
            (_, false, true) => conns.push(direct(&tile2, branch2)),
            (_, false, false) => {}
        }
        ftile_utils::add_connections(self, conns)
    }
}

impl Swimable for FtileIfWithLinks {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = SwimlaneSet::new();
        if let Some(in_) = self.in_ {
            result.insert(Some(in_));
        }
        result.extend(self.tile1.get_swimlanes());
        result.extend(self.tile2.get_swimlanes());
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.in_
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.in_
    }
}

impl Ftile for FtileIfWithLinks {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self.calculate_dimension_internal(string_bounder);
            if self
                .tile1
                .calculate_dimension(string_bounder)
                .has_point_out()
                || self
                    .tile2
                    .calculate_dimension(string_bounder)
                    .has_point_out()
            {
                dim_total
            } else {
                dim_total.without_point_out()
            }
        })
    }

    fn get_translate_for(&self, child: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        if same(child, self.tile1.as_ref()) {
            return self.get_translate_branch1(string_bounder);
        }
        if same(child, self.tile2.as_ref()) {
            return self.get_translate_branch2(string_bounder);
        }
        UTranslate::default()
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![
            Rc::clone(&self.diamond1),
            Rc::clone(&self.diamond2),
            Rc::clone(&self.tile1),
            Rc::clone(&self.tile2),
        ]
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.apply(self.get_translate_diamond1(string_bounder))
            .draw(&self.diamond1);
        ug.apply(self.get_translate_branch1(string_bounder))
            .draw(&self.tile1);
        ug.apply(self.get_translate_branch2(string_bounder))
            .draw(&self.tile2);
        ug.apply(self.get_translate_diamond2(string_bounder))
            .draw(&self.diamond2);
    }
}

/// From a side of the first diamond to a branch.
struct ConnectionHorizontalThenVertical {
    parent: Rc<FtileIfWithLinks>,
    base: AbstractConnection,
    color: Rainbow,
    using_arrow: Option<UPolygon>,
}

impl ConnectionHorizontalThenVertical {
    fn new(parent: &Rc<FtileIfWithLinks>, tile: &Rc<dyn Ftile>, branch: &BranchFtile<'_>) -> Self {
        Self {
            parent: Rc::clone(parent),
            base: AbstractConnection::new(Some(Rc::clone(&parent.diamond1)), Some(Rc::clone(tile))),
            color: branch.get_in_color(&parent.arrow_color),
            using_arrow: (!branch.is_empty()).then(|| parent.skin_param().arrows().as_to_down()),
        }
    }

    fn ftile2(&self) -> &Rc<dyn Ftile> {
        self.base.get_ftile2().expect("the connection enters a branch")
    }

    fn get_p1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let parent = &self.parent;
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let pt = if same(self.ftile2().as_ref(), parent.tile1.as_ref()) {
            dim_diamond1.get_point_d()
        } else {
            dim_diamond1.get_point_b()
        };
        parent.get_translate_diamond1(string_bounder).get_translated(pt)
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let tile = self.ftile2();
        self.parent
            .translate_branch(tile.as_ref(), string_bounder)
            .get_translated(tile.calculate_dimension(string_bounder).get_point_in())
    }

    fn snake(&self) -> Snake {
        Snake::create_with_decorations(
            self.parent.skin_param(),
            None,
            self.color.clone(),
            self.using_arrow.clone(),
        )
    }
}

impl Connection for ConnectionHorizontalThenVertical {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let p1 = self.get_p1(string_bounder);
        let p2 = self.get_p2(string_bounder);
        let mut snake = self.snake();
        snake.add_point(p1.x, p1.y);
        snake.add_point(p2.x, p1.y);
        snake.add_point(p2.x, p2.y);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionHorizontalThenVertical {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let mut p1 = self.get_p1(string_bounder);
        let mut p2 = self.get_p2(string_bounder);
        let original_direction = Direction::left_or_right(p1, p2);
        p1 = translate1.get_translated(p1);
        p2 = translate2.get_translated(p2);
        let new_direction = Direction::left_or_right(p1, p2);
        if original_direction != new_direction {
            let delta = if original_direction == Some(Direction::Right) {
                -1.0
            } else {
                1.0
            } * HEXAGON_HALF_SIZE;
            let dim_diamond1 = self.parent.diamond1.calculate_dimension(string_bounder);
            let mut small = Snake::create(self.parent.skin_param(), self.color.clone());
            small.add_point_at(p1);
            small.add_point(p1.x + delta, p1.y);
            small.add_point(p1.x + delta, p1.y + dim_diamond1.get_height() * 0.75);
            ug.draw(&small);
            p1 = small.get_last();
        }
        let mut snake = self.snake().with_merge(MergeStrategy::Limited);
        snake.add_point_at(p1);
        snake.add_point(p2.x, p1.y);
        snake.add_point_at(p2);
        ug.draw(&snake);
    }
}

/// The colours of an arrow out of a branch, the conditional's when it has none.
fn or_arrow_color(color: &Rainbow, arrow_color: &Rainbow) -> Rainbow {
    if color.size() == 0 {
        arrow_color.clone()
    } else {
        color.clone()
    }
}

/// From a branch down, then to a side of the second diamond.
struct ConnectionVerticalThenHorizontal {
    parent: Rc<FtileIfWithLinks>,
    base: AbstractConnection,
    my_arrow_color: Rainbow,
    branch_empty: bool,
}

impl ConnectionVerticalThenHorizontal {
    fn new(
        parent: &Rc<FtileIfWithLinks>,
        tile: &Rc<dyn Ftile>,
        my_arrow_color: &Rainbow,
        branch_empty: bool,
    ) -> Self {
        Self {
            parent: Rc::clone(parent),
            base: AbstractConnection::new(Some(Rc::clone(tile)), Some(Rc::clone(&parent.diamond2))),
            my_arrow_color: or_arrow_color(my_arrow_color, &parent.arrow_color),
            branch_empty,
        }
    }

    fn ftile1(&self) -> &Rc<dyn Ftile> {
        self.base.get_ftile1().expect("the connection leaves a branch")
    }

    fn get_p1(&self, string_bounder: &dyn StringBounder) -> Option<XPoint2D> {
        let tile = self.ftile1();
        let geo = tile.calculate_dimension(string_bounder);
        geo.has_point_out().then(|| {
            geo.translate(self.parent.translate_branch(tile.as_ref(), string_bounder))
                .get_point_out()
        })
    }

    fn get_p2(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let parent = &self.parent;
        let dim_diamond2 = parent.diamond2.calculate_dimension(string_bounder);
        let pt = if same(self.ftile1().as_ref(), parent.tile1.as_ref()) {
            dim_diamond2.get_point_d()
        } else {
            dim_diamond2.get_point_b()
        };
        parent.get_translate_diamond2(string_bounder).get_translated(pt)
    }

    fn arrow(&self, x1: f64, x2: f64) -> UPolygon {
        let arrows = self.parent.skin_param().arrows();
        if x2 > x1 {
            arrows.as_to_right()
        } else {
            arrows.as_to_left()
        }
    }
}

impl Connection for ConnectionVerticalThenHorizontal {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let Some(p1) = self.get_p1(string_bounder) else {
            return;
        };
        let p2 = self.get_p2(string_bounder);
        let mut snake = Snake::create_with_end(
            self.parent.skin_param(),
            self.my_arrow_color.clone(),
            self.arrow(p1.x, p2.x),
        );
        if self.branch_empty {
            snake = snake.emphasize_direction(Direction::Down);
        }
        snake.add_point(p1.x, p1.y);
        snake.add_point(p1.x, p2.y);
        snake.add_point(p2.x, p2.y);
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionVerticalThenHorizontal {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let Some(p1) = self.get_p1(string_bounder) else {
            return;
        };
        let p2 = self.get_p2(string_bounder);
        let original_direction = Direction::left_or_right(p1, p2);
        let (x1, x2) = (p1.x, p2.x);
        let mp1a = translate1.get_translated(p1);
        let mp2b = translate2.get_translated(p2);
        let new_direction = Direction::left_or_right(mp1a, mp2b);
        let arrow = self.arrow(x1, x2);
        let skin_param = self.parent.skin_param();
        let delta = if x2 > x1 { -1.0 } else { 1.0 } * 1.5 * HEXAGON_HALF_SIZE;
        let corner = if original_direction == new_direction {
            let mp2bc = XPoint2D::new(mp2b.x + delta, mp2b.y);
            let middle = f64::midpoint(mp1a.y, mp2b.y);
            let mut snake = Snake::create(skin_param, self.my_arrow_color.clone())
                .with_merge(MergeStrategy::Limited);
            snake.add_point_at(mp1a);
            snake.add_point(mp1a.x, middle);
            snake.add_point(mp2bc.x, middle);
            snake.add_point_at(mp2bc);
            ug.draw(&snake);
            mp2bc
        } else {
            let mp2bb = XPoint2D::new(mp2b.x + delta, mp2b.y - 1.5 * HEXAGON_HALF_SIZE);
            let mut snake = Snake::create(skin_param, self.my_arrow_color.clone())
                .with_merge(MergeStrategy::Limited);
            snake.add_point_at(mp1a);
            snake.add_point(mp1a.x, mp2bb.y);
            snake.add_point_at(mp2bb);
            ug.draw(&snake);
            mp2bb
        };
        let mut small = Snake::create_with_end(skin_param, self.my_arrow_color.clone(), arrow)
            .with_merge(MergeStrategy::Limited);
        small.add_point_at(corner);
        small.add_point(corner.x, mp2b.y);
        small.add_point_at(mp2b);
        ug.draw(&small);
    }
}

/// From the one branch that goes on, down to where the conditional is left.
struct ConnectionVerticalThenHorizontalDirect {
    parent: Rc<FtileIfWithLinks>,
    base: AbstractConnection,
    my_arrow_color: Rainbow,
    branch_empty: bool,
}

impl ConnectionVerticalThenHorizontalDirect {
    fn new(
        parent: &Rc<FtileIfWithLinks>,
        tile: &Rc<dyn Ftile>,
        my_arrow_color: &Rainbow,
        branch_empty: bool,
    ) -> Self {
        Self {
            parent: Rc::clone(parent),
            base: AbstractConnection::new(Some(Rc::clone(tile)), Some(Rc::clone(&parent.diamond2))),
            my_arrow_color: or_arrow_color(my_arrow_color, &parent.arrow_color),
            branch_empty,
        }
    }

    fn get_p1(&self, string_bounder: &dyn StringBounder) -> Option<XPoint2D> {
        let tile = self.base.get_ftile1().expect("the connection leaves a branch");
        let geo = tile.calculate_dimension(string_bounder);
        geo.has_point_out().then(|| {
            geo.translate(self.parent.translate_branch(tile.as_ref(), string_bounder))
                .get_point_out()
        })
    }
}

impl Connection for ConnectionVerticalThenHorizontalDirect {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.parent.calculate_dimension_internal(string_bounder);
        let Some(p1) = self.get_p1(string_bounder) else {
            return;
        };
        let p2 = XPoint2D::new(dim_total.get_left(), dim_total.get_height());
        let mut snake = Snake::create(self.parent.skin_param(), self.my_arrow_color.clone());
        if self.branch_empty {
            snake = snake.emphasize_direction(Direction::Down);
        }
        snake.add_point(p1.x, p1.y);
        snake.add_point(p1.x, p2.y);
        snake.add_point(p2.x, p2.y);
        snake.add_point(p2.x, dim_total.get_height());
        ug.draw(&snake);
    }

    fn as_translatable(&self) -> Option<&dyn ConnectionTranslatable> {
        Some(self)
    }
}

impl ConnectionTranslatable for ConnectionVerticalThenHorizontalDirect {
    fn draw_translate(&self, ug: &UGraphic, translate1: UTranslate, translate2: UTranslate) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.parent.calculate_dimension_internal(string_bounder);
        let Some(p1) = self.get_p1(string_bounder) else {
            return;
        };
        let p2 = XPoint2D::new(
            dim_total.get_left(),
            dim_total.get_height() - HEXAGON_HALF_SIZE,
        );
        let mp1a = translate1.get_translated(p1);
        let mp2b = translate2.get_translated(p2);
        let mut snake = Snake::create(self.parent.skin_param(), self.my_arrow_color.clone())
            .with_merge(MergeStrategy::Limited);
        snake.add_point_at(mp1a);
        snake.add_point(mp1a.x, mp2b.y);
        snake.add_point_at(mp2b);
        snake.add_point(mp2b.x, dim_total.get_height());
        ug.draw(&snake);
    }
}

/// With `conditionEndStyle hline`: from a branch straight down to the line.
struct ConnectionVerticalOut {
    parent: Rc<FtileIfWithLinks>,
    base: AbstractConnection,
}

impl ConnectionVerticalOut {
    fn new(parent: &Rc<FtileIfWithLinks>, tile: &Rc<dyn Ftile>) -> Self {
        Self {
            parent: Rc::clone(parent),
            base: AbstractConnection::new(Some(Rc::clone(tile)), None),
        }
    }
}

impl Connection for ConnectionVerticalOut {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let tile = self.base.get_ftile1().expect("the connection leaves a branch");
        let geo = tile.calculate_dimension(string_bounder);
        if !geo.has_point_out() {
            return;
        }
        let p1 = geo
            .translate(self.parent.translate_branch(tile.as_ref(), string_bounder))
            .get_point_out();
        let total_height = self
            .parent
            .calculate_dimension_internal(string_bounder)
            .get_height();
        let skin_param = self.parent.skin_param();
        let mut snake = Snake::create_with_end(
            skin_param,
            self.parent.arrow_color.clone(),
            skin_param.arrows().as_to_down(),
        );
        snake.add_point_at(p1);
        snake.add_point(p1.x, total_height);
        ug.draw(&snake);
    }
}

/// With `conditionEndStyle hline`: the line joining the branches.
struct ConnectionHline {
    parent: Rc<FtileIfWithLinks>,
    base: AbstractConnection,
}

impl Connection for ConnectionHline {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let parent = &self.parent;
        let total_dim = parent.calculate_dimension_internal(ug.string_bounder());
        let all_tiles = [Rc::clone(&parent.tile1), Rc::clone(&parent.tile2)];
        let Some((min_x, max_x)) = hline_extent(
            ug,
            total_dim.get_width(),
            &all_tiles,
            None,
            |tile, string_bounder| parent.get_translate_for(tile, string_bounder),
        ) else {
            return;
        };
        let mut snake = Snake::create(parent.skin_param(), parent.arrow_color.clone())
            .with_merge(MergeStrategy::None);
        snake.add_point(min_x, total_dim.get_height());
        snake.add_point(max_x, total_dim.get_height());
        ug.draw(&snake);
    }
}
