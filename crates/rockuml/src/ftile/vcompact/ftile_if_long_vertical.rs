//! An `if` with `elseif`s under `!pragma useVerticalIf on`: the conditions one below the other, each
//! branch right of its condition (PlantUML's `FtileIfLongVertical`).

use std::rc::Rc;

use super::create0_or_empty;
use crate::color::HColor;
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{BranchFtile, LinkRendering, SwimlaneId, SwimlaneSet};
use crate::ftile::vertical::{FtileDiamond, FtileDiamondInside2};
use crate::ftile::{
    AbstractConnection, AbstractFtile, Connection, Ftile, FtileFactory, FtileGeometry,
    FtileMargedWest, FtileMinWidthCentered, Snake, Swimable, ftile_utils, same,
};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};

const Y_SEPARATION: f64 = 20.0;
const MARGINY1: f64 = 30.0;

pub(crate) struct FtileIfLongVertical {
    base: AbstractFtile,
    tiles: Vec<Rc<dyn Ftile>>,
    tile2: Rc<dyn Ftile>,
    diamonds: Vec<Rc<dyn Ftile>>,
    last_diamond: Rc<dyn Ftile>,
    arrow_color: Rainbow,
}

impl FtileIfLongVertical {
    /// The conditional of `thens`, ending with `branch2`, its `else`.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's create")]
    pub(crate) fn create(
        swimlane: Option<SwimlaneId>,
        back_color: &HColor,
        ftile_factory: &dyn FtileFactory,
        thens: &[BranchFtile<'_>],
        branch2: &BranchFtile<'_>,
        top_inlink_rendering: &LinkRendering,
        style_arrow: &Style,
        style_diamond: &Style,
    ) -> Rc<dyn Ftile> {
        let skin_param = ftile_factory.skin_param();
        let string_bounder = ftile_factory.get_string_bounder();
        let fc_arrow = style_arrow.font_configuration();
        let border_color = style_diamond.value(PName::LineColor).as_color();
        let arrow_color = Rainbow::build_from_style(style_arrow);
        let create = |display: Option<&Display>, alignment: HorizontalAlignment| {
            create0_or_empty(
                display,
                &fc_arrow,
                alignment,
                skin_param,
                0.0,
                CreoleMode::Full,
            )
        };
        let mut diamonds: Vec<Rc<dyn Ftile>> = Vec::new();
        let mut west: f64 = 10.0;
        for branch in thens {
            let tb1 = create(
                branch.get_display_positive().as_ref(),
                HorizontalAlignment::Left,
            );
            let tb_test = create(
                branch.branch.label_test.as_ref(),
                skin_param.get_default_text_alignment(HorizontalAlignment::Left),
            );
            let diamond = FtileDiamondInside2::new(
                tb_test,
                Rc::clone(skin_param),
                back_color.clone(),
                border_color.clone(),
                swimlane,
            );
            diamonds.push(Rc::new(diamond.with_east(tb1)));
            if let Some(inlabel) = branch.branch.get_inlabel() {
                let tb_inlabel = create(Some(inlabel), HorizontalAlignment::Left);
                west = west.max(tb_inlabel.calculate_dimension(string_bounder).width);
            }
        }
        let then_tiles: Vec<Rc<dyn Ftile>> = thens
            .iter()
            .map(|branch| {
                Rc::new(FtileMargedWest::new(Rc::clone(&branch.ftile), west)) as Rc<dyn Ftile>
            })
            .collect();
        let last_diamond: Rc<dyn Ftile> = Rc::new(FtileDiamond::new(
            Rc::clone(skin_param),
            back_color.clone(),
            border_color,
            swimlane,
        ));
        let result = Rc::new(Self {
            base: AbstractFtile::new(Rc::clone(skin_param)),
            tiles: then_tiles,
            tile2: Rc::new(FtileMinWidthCentered::new(Rc::clone(&branch2.ftile), 30.0)),
            diamonds,
            last_diamond,
            arrow_color,
        });
        let tb2 = create(
            branch2.get_display_positive().as_ref(),
            HorizontalAlignment::Left,
        );
        let inlabels = thens
            .iter()
            .skip(1)
            .map(|branch| {
                branch
                    .branch
                    .get_inlabel()
                    .map(|inlabel| create(Some(inlabel), HorizontalAlignment::Left))
            })
            .collect();
        let in_colors = thens
            .iter()
            .map(|branch| branch.get_in_color(&result.arrow_color))
            .collect();
        let conns = result.connections(in_colors, inlabels, top_inlink_rendering, tb2);
        ftile_utils::add_connections(result, conns)
    }

    /// The arrows: `in_colors` into the branches, `inlabels` into the conditions after the first, `tb2`
    /// into the `else`.
    fn connections(
        self: &Rc<Self>,
        in_colors: Vec<Rainbow>,
        inlabels: Vec<Option<Rc<dyn TextBlock>>>,
        top_inlink_rendering: &LinkRendering,
        tb2: Rc<dyn TextBlock>,
    ) -> Vec<Rc<dyn Connection>> {
        let (tiles, diamonds) = (&self.tiles, &self.diamonds);
        let arrow_color = &self.arrow_color;
        let mut conns = Vec::new();
        for ((tile, diamond), color) in tiles.iter().zip(diamonds).zip(in_colors) {
            conns.push(self.connection(
                ConnectionKind::VerticalIn,
                Some(diamond),
                Some(tile),
                color,
                None,
            ));
        }
        for (pair, tb_inlabel) in diamonds.windows(2).zip(inlabels) {
            conns.push(self.connection(
                ConnectionKind::Vertical,
                Some(&pair[0]),
                Some(&pair[1]),
                arrow_color.clone(),
                tb_inlabel,
            ));
        }
        if let Some(first) = tiles.first() {
            conns.push(self.connection(
                ConnectionKind::ThenOut,
                Some(first),
                Some(&self.last_diamond),
                arrow_color.clone(),
                None,
            ));
        }
        for tile in tiles.iter().skip(1) {
            conns.push(self.connection(
                ConnectionKind::ThenOutConnect,
                Some(tile),
                Some(&self.last_diamond),
                arrow_color.clone(),
                None,
            ));
        }
        let top_in_color = top_inlink_rendering.get_rainbow_or(arrow_color);
        conns.push(self.connection(
            ConnectionKind::In,
            None,
            diamonds.first(),
            top_in_color.clone(),
            None,
        ));
        conns.push(self.connection(
            ConnectionKind::LastElse,
            diamonds.last(),
            Some(&self.tile2),
            top_in_color,
            Some(tb2),
        ));
        conns.push(self.connection(
            ConnectionKind::LastElseOut,
            Some(&self.tile2),
            Some(&self.last_diamond),
            arrow_color.clone(),
            None,
        ));
        conns
    }

    fn connection(
        self: &Rc<Self>,
        kind: ConnectionKind,
        tile1: Option<&Rc<dyn Ftile>>,
        tile2: Option<&Rc<dyn Ftile>>,
        color: Rainbow,
        label: Option<Rc<dyn TextBlock>>,
    ) -> Rc<dyn Connection> {
        Rc::new(ConnectionLongVertical {
            parent: Rc::clone(self),
            base: AbstractConnection::new(tile1.cloned(), tile2.cloned()),
            kind,
            color,
            label,
        })
    }

    fn get_translate_diamond(
        &self,
        diamond: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let all_diamonds_width = self.all_diamonds_width(string_bounder);
        let Some(idx) = self
            .diamonds
            .iter()
            .position(|other| same(other.as_ref(), diamond))
        else {
            return UTranslate::default();
        };
        let y1 = self.get_translate_dy(idx, string_bounder);
        UTranslate::new(
            (all_diamonds_width - diamond.calculate_dimension(string_bounder).get_width()) / 2.0,
            y1,
        )
    }

    fn get_translate_last_diamond(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_last = self.last_diamond.calculate_dimension(string_bounder);
        let x = (dim_total.get_width() - dim_last.get_width()) / 2.0;
        UTranslate::new(x, dim_total.get_height() - dim_last.get_height())
    }

    fn get_translate1(
        &self,
        candidate: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        let Some(idx) = self
            .tiles
            .iter()
            .position(|other| same(other.as_ref(), candidate))
        else {
            return UTranslate::default();
        };
        let y1 = self.get_translate_dy(idx, string_bounder);
        let diam = self.diamonds[idx].calculate_dimension(string_bounder);
        let dim1 = candidate.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let all_diamonds_width = self.all_diamonds_width(string_bounder);
        let x = all_diamonds_width
            + (dim_total.get_width() - all_diamonds_width - dim1.get_width()) / 2.0;
        UTranslate::new(x, y1 + diam.get_height())
    }

    fn get_translate_dy(&self, idx: usize, string_bounder: &dyn StringBounder) -> f64 {
        let mut y1 = MARGINY1;
        for (tile, diamond) in self.tiles.iter().zip(&self.diamonds).take(idx) {
            let dim1 = tile.calculate_dimension(string_bounder);
            let diam = diamond.calculate_dimension(string_bounder);
            y1 += dim1.get_height() + diam.get_height() + Y_SEPARATION;
        }
        y1
    }

    fn get_translate2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let y1 = self.get_translate_dy(self.tiles.len(), string_bounder);
        let dim2 = self.tile2.calculate_dimension(string_bounder);
        let dim_total = self.calculate_dimension_internal(string_bounder);
        UTranslate::new((dim_total.get_width() - dim2.get_width()) / 2.0, y1)
    }

    fn calculate_dimension_internal(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let mut col1: f64 = 0.0;
        let mut col2: f64 = 0.0;
        let mut height = MARGINY1;
        for (tile, diamond) in self.tiles.iter().zip(&self.diamonds) {
            let dim1 = tile.calculate_dimension(string_bounder);
            let diamond_dim = diamond.calculate_dimension(string_bounder);
            height += diamond_dim.get_height() + dim1.get_height();
            col1 = col1.max(diamond_dim.get_width());
            col2 = col2.max(dim1.get_width());
        }
        let width = col1 + col2;
        let dim_tile2 = self.tile2.calculate_dimension(string_bounder);
        let dim_last_diamond = self.last_diamond.calculate_dimension(string_bounder);
        let last_else_arrow_height = 40.0;
        let result = FtileGeometry::new(width, height, width / 2.0, 0.0)
            .append_bottom(dim_tile2)
            .add_dim(
                0.0,
                Y_SEPARATION * self.tiles.len() as f64
                    + last_else_arrow_height
                    + dim_last_diamond.get_height(),
            );
        FtileGeometry::from_dim(result.dimension(), result.get_width() / 2.0, 0.0)
    }

    fn all_diamonds_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.diamonds.iter().fold(0.0, |width: f64, diamond| {
            width.max(diamond.calculate_dimension(string_bounder).get_width())
        })
    }
}

impl Swimable for FtileIfLongVertical {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = SwimlaneSet::new();
        if let Some(in_) = self.get_swimlane_in() {
            result.insert(Some(in_));
        }
        for tile in &self.tiles {
            result.extend(tile.get_swimlanes());
        }
        result.extend(self.tile2.get_swimlanes());
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.tiles.first().and_then(|tile| tile.get_swimlane_in())
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.get_swimlane_in()
    }
}

impl Ftile for FtileIfLongVertical {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self
                .calculate_dimension_internal(string_bounder)
                .dimension();
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

    fn get_translate_for(
        &self,
        child: &dyn Ftile,
        string_bounder: &dyn StringBounder,
    ) -> UTranslate {
        if same(child, self.tile2.as_ref()) {
            return self.get_translate2(string_bounder);
        }
        if same(child, self.last_diamond.as_ref()) {
            return self.get_translate_last_diamond(string_bounder);
        }
        if self.tiles.iter().any(|tile| same(tile.as_ref(), child)) {
            return self.get_translate1(child, string_bounder);
        }
        self.get_translate_diamond(child, string_bounder)
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.tiles.iter().chain([&self.tile2]).cloned().collect()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        for tile1 in &self.tiles {
            ug.apply(self.get_translate1(tile1.as_ref(), string_bounder))
                .draw(tile1);
        }
        for diam in &self.diamonds {
            ug.apply(self.get_translate_diamond(diam.as_ref(), string_bounder))
                .draw(diam);
        }
        ug.apply(self.get_translate2(string_bounder))
            .draw(&self.tile2);
        ug.apply(self.get_translate_last_diamond(string_bounder))
            .draw(&self.last_diamond);
    }
}

/// PlantUML's inner connection classes of `FtileIfLongVertical`, none of which crosses lanes.
#[derive(Clone, Copy)]
enum ConnectionKind {
    /// Into the first condition.
    In,
    /// From the right of a condition into its branch.
    VerticalIn,
    /// From a condition down to the next.
    Vertical,
    /// From the last condition down into the `else`.
    LastElse,
    /// From the `else` down into the last diamond.
    LastElseOut,
    /// From the first branch round the right into the last diamond.
    ThenOut,
    /// From another branch to the line on the right.
    ThenOutConnect,
}

struct ConnectionLongVertical {
    parent: Rc<FtileIfLongVertical>,
    base: AbstractConnection,
    kind: ConnectionKind,
    color: Rainbow,
    label: Option<Rc<dyn TextBlock>>,
}

impl ConnectionLongVertical {
    /// The arrow, its head as the kind of connection has it, before its points are added.
    fn snake(&self) -> Snake {
        let skin_param = self.parent.skin_param();
        let arrows = skin_param.arrows();
        let head = match self.kind {
            ConnectionKind::ThenOut => arrows.as_to_left(),
            ConnectionKind::ThenOutConnect => arrows.as_to_right(),
            _ => arrows.as_to_down(),
        };
        let snake = Snake::create_with_end(skin_param, self.color.clone(), head);
        match self.kind {
            ConnectionKind::Vertical | ConnectionKind::LastElse => {
                snake.with_label_vertical(self.label.clone(), VerticalAlignment::Center)
            }
            _ => snake,
        }
    }

    /// The point out of the branch `tile`, when the flow goes on below it.
    fn branch_out(
        &self,
        tile: &Rc<dyn Ftile>,
        string_bounder: &dyn StringBounder,
    ) -> Option<XPoint2D> {
        let dim1 = tile.calculate_dimension(string_bounder);
        dim1.has_point_out().then(|| {
            self.parent
                .get_translate1(tile.as_ref(), string_bounder)
                .get_translated(dim1.get_point_out())
        })
    }

    /// From the first branch round the right, into the middle of the last diamond.
    fn then_out_points(
        &self,
        tile: &Rc<dyn Ftile>,
        last_diamond: &Rc<dyn Ftile>,
        string_bounder: &dyn StringBounder,
    ) -> Option<Vec<XPoint2D>> {
        let parent = &self.parent;
        let p1 = self.branch_out(tile, string_bounder)?;
        let dim_last_diamond = last_diamond.calculate_dimension(string_bounder);
        let p2 = parent
            .get_translate_last_diamond(string_bounder)
            .get_translated(dim_last_diamond.get_point_in());
        let p2 = UTranslate::new(
            dim_last_diamond.get_width() / 2.0,
            dim_last_diamond.get_height() / 2.0,
        )
        .get_translated(p2);
        let width = parent
            .calculate_dimension_internal(string_bounder)
            .get_width();
        Some(vec![
            p1,
            XPoint2D::new(p1.x, p1.y + 15.0),
            XPoint2D::new(width, p1.y + 15.0),
            XPoint2D::new(width, p2.y),
            p2,
        ])
    }

    /// The points the arrow goes through; none when it is not drawn.
    fn points(&self, string_bounder: &dyn StringBounder) -> Option<Vec<XPoint2D>> {
        let parent = &self.parent;
        let point_in = |tile: &Rc<dyn Ftile>, translate: UTranslate| {
            translate.get_translated(tile.calculate_dimension(string_bounder).get_point_in())
        };
        let point_out = |tile: &Rc<dyn Ftile>, translate: UTranslate| {
            translate.get_translated(tile.calculate_dimension(string_bounder).get_point_out())
        };
        let dim_total = || parent.calculate_dimension_internal(string_bounder);
        Some(
            match (self.kind, self.base.get_ftile1(), self.base.get_ftile2()) {
                (ConnectionKind::In, _, Some(diamond)) => {
                    let p2 = point_in(
                        diamond,
                        parent.get_translate_diamond(diamond.as_ref(), string_bounder),
                    );
                    let p1 = dim_total().get_point_in();
                    let middle = f64::midpoint(p1.y, p2.y);
                    vec![
                        p1,
                        XPoint2D::new(p1.x, middle),
                        XPoint2D::new(p2.x, middle),
                        p2,
                    ]
                }
                (ConnectionKind::VerticalIn, Some(diamond), Some(tile)) => {
                    let dim_diamond1 = diamond.calculate_dimension(string_bounder);
                    let p1 = parent
                        .get_translate_diamond(diamond.as_ref(), string_bounder)
                        .get_translated(XPoint2D::new(
                            dim_diamond1.get_width(),
                            dim_diamond1.get_height() / 2.0,
                        ));
                    let p2 = point_in(tile, parent.get_translate1(tile.as_ref(), string_bounder));
                    vec![p1, XPoint2D::new(p2.x, p1.y), p2]
                }
                (ConnectionKind::Vertical, Some(diamond1), Some(diamond2)) => vec![
                    point_out(
                        diamond1,
                        parent.get_translate_for(diamond1.as_ref(), string_bounder),
                    ),
                    point_in(
                        diamond2,
                        parent.get_translate_for(diamond2.as_ref(), string_bounder),
                    ),
                ],
                (ConnectionKind::LastElse, Some(diamond), Some(tile2)) => {
                    let p1 = point_out(
                        diamond,
                        parent.get_translate_diamond(diamond.as_ref(), string_bounder),
                    );
                    let p2 = point_in(tile2, parent.get_translate2(string_bounder));
                    vec![
                        p1,
                        XPoint2D::new(p1.x, p2.y - 15.0),
                        XPoint2D::new(p2.x, p2.y - 15.0),
                        p2,
                    ]
                }
                (ConnectionKind::LastElseOut, Some(tile2), Some(last_diamond)) => {
                    if !tile2.calculate_dimension(string_bounder).has_point_out() {
                        return None;
                    }
                    let p1 = point_out(tile2, parent.get_translate2(string_bounder));
                    let p2 = point_in(
                        last_diamond,
                        parent.get_translate_last_diamond(string_bounder),
                    );
                    vec![
                        p1,
                        XPoint2D::new(p1.x, p2.y - 15.0),
                        XPoint2D::new(p2.x, p2.y - 15.0),
                        p2,
                    ]
                }
                (ConnectionKind::ThenOut, Some(tile), Some(last_diamond)) => {
                    return self.then_out_points(tile, last_diamond, string_bounder);
                }
                (ConnectionKind::ThenOutConnect, Some(tile), Some(_)) => {
                    let p1 = self.branch_out(tile, string_bounder)?;
                    let p2 = XPoint2D::new(dim_total().get_width(), p1.y + 15.0);
                    vec![p1, XPoint2D::new(p1.x, p2.y), p2]
                }
                _ => return None,
            },
        )
    }
}

impl Connection for ConnectionLongVertical {
    fn get_ftile1(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile1()
    }

    fn get_ftile2(&self) -> Option<&Rc<dyn Ftile>> {
        self.base.get_ftile2()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let Some(points) = self.points(ug.string_bounder()) else {
            return;
        };
        let mut snake = self.snake();
        for point in points {
            snake.add_point_at(point);
        }
        ug.draw(&snake);
    }
}
