//! An `if` with an `else`, both branches side by side between two diamonds (PlantUML's `FtileIfWithLinks`,
//! with its bases `FtileIfWithDiamonds` and `FtileIfNude`, which nothing else builds).

use std::cell::OnceCell;
use std::rc::Rc;

use crate::decoration::Rainbow;
use crate::diagram::activity3::{
    BranchFtile, NotePosition, PositionedNote, SwimlaneId, SwimlaneSet,
};
use crate::direction::Direction;
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::vcompact::connection_hline::hline_extent;
use crate::ftile::vcompact::note_sheet::NoteSheet;
use crate::ftile::vertical::FtileDiamond;
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, ConnectionTranslatable, Ftile, FtileGeometry,
    MergeStrategy, Snake, Swimable, downcast, ftile_utils, same,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D, XPoint2D};
use crate::klimt::shape::UPolygon;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::ConditionEndStyle;
use crate::svek::image::Opale;

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
    /// The first note on the left of the diamond, and the first on its right.
    opale_left: Option<Opale<'static>>,
    opale_right: Option<Opale<'static>>,
    /// The room the notes take: left of the tile, above it, right of it.
    x_delta_note: f64,
    y_delta_note: f64,
    supp_width_node: f64,
    arrow_color: Rainbow,
    condition_end_style: ConditionEndStyle,
}

/// A note on a condition, in the activity note style, its own colours over it
/// (`FtileIfWithDiamonds.createOpale`).
pub(crate) fn create_opale(note: &PositionedNote, skin_param: &SkinParam) -> Opale<'static> {
    let style = StyleSignature::of(&[
        SName::Root,
        SName::Element,
        SName::ActivityDiagram,
        SName::Note,
    ])
    .get_merged_style(&skin_param.current_style_builder())
    .eventually_override_colors(&note.colors);
    let text = NoteSheet::new(
        &note.display,
        &style.font_configuration(),
        skin_param.note_text_alignment(HorizontalAlignment::Left),
        style.wrap_width(),
        skin_param,
    );
    Opale::new(
        style.value(PName::LineColor).as_color(),
        style.value(PName::BackGroundColor).as_color(),
        Box::new(text),
        style.stroke(),
        0.0,
    )
}

impl FtileIfWithLinks {
    /// The tile, with room for the first note of `notes` left of the diamond and the first right of it.
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
        string_bounder: &dyn StringBounder,
        notes: &[PositionedNote],
    ) -> Self {
        let mut result = Self {
            base: AbstractFtile::new(skin_param),
            dimension_internal: OnceCell::new(),
            tile1,
            tile2,
            diamond1,
            diamond2,
            in_,
            opale_left: None,
            opale_right: None,
            x_delta_note: 0.0,
            y_delta_note: 0.0,
            supp_width_node: 0.0,
            arrow_color,
            condition_end_style,
        };
        for note in notes {
            match note.note_position {
                NotePosition::Left if result.opale_left.is_none() => {
                    let opale = create_opale(note, result.skin_param());
                    let pos1 = result.get_translate_diamond1(string_bounder).dx;
                    let dim_opale = opale.calculate_dimension(string_bounder);
                    if dim_opale.width > pos1 {
                        result.x_delta_note = dim_opale.width - pos1;
                    }
                    result.y_delta_note = result.y_delta_note.max(dim_opale.height);
                    result.opale_left = Some(opale);
                }
                NotePosition::Right if result.opale_right.is_none() => {
                    let opale = create_opale(note, result.skin_param());
                    let dim_opale = opale.calculate_dimension(string_bounder);
                    let pos1 = result.get_translate_diamond1(string_bounder).dx
                        + result
                            .diamond1
                            .calculate_dimension(string_bounder)
                            .get_width()
                        + dim_opale.width;
                    let pos2 = result
                        .calculate_dimension_internal_slow(string_bounder)
                        .get_width();
                    if pos1 > pos2 {
                        result.supp_width_node = pos1 - pos2;
                    }
                    result.y_delta_note = result.y_delta_note.max(dim_opale.height);
                    result.opale_right = Some(opale);
                }
                NotePosition::Left | NotePosition::Right => continue,
                _ => {}
            }
            result.dimension_internal.take();
        }
        result
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
        downcast::<FtileDiamond>(self.diamond2.as_ref()).map_or(0.0, |diamond2| {
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

    fn calculate_dimension_internal_slow(
        &self,
        string_bounder: &dyn StringBounder,
    ) -> FtileGeometry {
        let dim1 = self.diamond1.calculate_dimension(string_bounder);
        let dim2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_nude = self.calculate_dimension_nude(string_bounder);
        let all = dim1.append_bottom(dim_nude).append_bottom(dim2);
        let delta_height = self.get_ydelta1a()
            + self.get_ydelta1b(string_bounder)
            + self.get_ydelta_for_labels(string_bounder);
        all.add_dim(0.0, delta_height).inc_in_y(self.y_delta_note)
    }

    /// The branches side by side (`FtileIfNude.calculateDimensionInternalSlow`).
    fn calculate_dimension_nude(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let dim1 = self.tile1.calculate_dimension(string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        let inner_margin = self.width_inner(string_bounder);
        let width = self.x_delta_note
            + dim1.get_left()
            + inner_margin
            + (dim2.get_width() - dim2.get_left())
            + self.supp_width_node;
        let height = self.y_delta_note + dim1.dimension().merge_lr(dim2.dimension()).height;
        FtileGeometry::with_out(
            width,
            height,
            self.x_delta_note + dim1.get_left() + inner_margin / 2.0,
            self.y_delta_note,
            height,
        )
    }

    fn get_translate_branch1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(self.x_delta_note, self.y_delta_note).compose(UTranslate::new(
            0.0,
            dim_diamond1.get_height() + self.get_ydelta1a(),
        ))
    }

    fn get_translate_branch2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_width() - dim2.get_width() - self.supp_width_node,
            self.y_delta_note,
        )
        .compose(UTranslate::new(
            0.0,
            dim_diamond1.get_height() + self.get_ydelta1a(),
        ))
    }

    fn get_translate_diamond1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_left() - dim_diamond1.get_left(),
            self.y_delta_note,
        )
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
            Rc::new(ConnectionHorizontalThenVertical::new(
                &self, &tile1, branch1,
            )),
            Rc::new(ConnectionHorizontalThenVertical::new(
                &self, &tile2, branch2,
            )),
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
    fn skin_param(&self) -> &Rc<SkinParam> {
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

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
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
        let translate_diamond1 = self.get_translate_diamond1(string_bounder);
        if let Some(opale) = &self.opale_left {
            let x_opale = translate_diamond1.dx - opale.calculate_dimension(string_bounder).width;
            opale.draw_u(&ug.apply(UTranslate::new(x_opale, 0.0)));
        }
        if let Some(opale) = &self.opale_right {
            let x_opale = translate_diamond1.dx
                + self
                    .diamond1
                    .calculate_dimension(string_bounder)
                    .get_width();
            opale.draw_u(&ug.apply(UTranslate::new(x_opale, 0.0)));
        }
        ug.apply(translate_diamond1).draw(&self.diamond1);
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
        self.base
            .get_ftile2()
            .expect("the connection enters a branch")
    }

    fn get_p1(&self, string_bounder: &dyn StringBounder) -> XPoint2D {
        let parent = &self.parent;
        let dim_diamond1 = parent.diamond1.calculate_dimension(string_bounder);
        let pt = if same(self.ftile2().as_ref(), parent.tile1.as_ref()) {
            dim_diamond1.get_point_d()
        } else {
            dim_diamond1.get_point_b()
        };
        parent
            .get_translate_diamond1(string_bounder)
            .get_translated(pt)
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
        self.base
            .get_ftile1()
            .expect("the connection leaves a branch")
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
        parent
            .get_translate_diamond2(string_bounder)
            .get_translated(pt)
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
        let from = translate1.get_translated(p1);
        let to = translate2.get_translated(p2);
        let new_direction = Direction::left_or_right(from, to);
        let arrow = self.arrow(x1, x2);
        let skin_param = self.parent.skin_param();
        let delta = if x2 > x1 { -1.0 } else { 1.0 } * 1.5 * HEXAGON_HALF_SIZE;
        let corner = if original_direction == new_direction {
            let elbow = XPoint2D::new(to.x + delta, to.y);
            let middle = f64::midpoint(from.y, to.y);
            let mut snake = Snake::create(skin_param, self.my_arrow_color.clone())
                .with_merge(MergeStrategy::Limited);
            snake.add_point_at(from);
            snake.add_point(from.x, middle);
            snake.add_point(elbow.x, middle);
            snake.add_point_at(elbow);
            ug.draw(&snake);
            elbow
        } else {
            let elbow = XPoint2D::new(to.x + delta, to.y - 1.5 * HEXAGON_HALF_SIZE);
            let mut snake = Snake::create(skin_param, self.my_arrow_color.clone())
                .with_merge(MergeStrategy::Limited);
            snake.add_point_at(from);
            snake.add_point(from.x, elbow.y);
            snake.add_point_at(elbow);
            ug.draw(&snake);
            elbow
        };
        let mut small = Snake::create_with_end(skin_param, self.my_arrow_color.clone(), arrow)
            .with_merge(MergeStrategy::Limited);
        small.add_point_at(corner);
        small.add_point(corner.x, to.y);
        small.add_point_at(to);
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
        let tile = self
            .base
            .get_ftile1()
            .expect("the connection leaves a branch");
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
        let from = translate1.get_translated(p1);
        let to = translate2.get_translated(p2);
        let mut snake = Snake::create(self.parent.skin_param(), self.my_arrow_color.clone())
            .with_merge(MergeStrategy::Limited);
        snake.add_point_at(from);
        snake.add_point(from.x, to.y);
        snake.add_point_at(to);
        snake.add_point(to.x, dim_total.get_height());
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
        let tile = self
            .base
            .get_ftile1()
            .expect("the connection leaves a branch");
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
