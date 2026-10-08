//! A `switch`: its cases side by side between two diamonds (PlantUML's `FtileSwitchWithDiamonds`, with its
//! base `FtileSwitchNude`, which nothing else builds). Its subclasses `FtileSwitchWithOneLink` and
//! `FtileSwitchWithManyLinks` add the arrows, in their files.

use std::cell::OnceCell;
use std::rc::Rc;

use crate::decoration::Rainbow;
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable, same};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;

const X_SEPARATION: f64 = 20.0;
const SUPP15: f64 = 15.0;

/// Whether the first diamond is wider than the cases between the first and the last.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// The cases spread out under the diamond.
    BigDiamond,
    SmallDiamond,
}

/// Which subclass of PlantUML's `FtileSwitchWithDiamonds` the tile is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SwitchLinks {
    /// `FtileSwitchWithOneLink`: a single case.
    OneLink,
    /// `FtileSwitchWithManyLinks`, which leaves room above the cases for their labels.
    ManyLinks,
}

pub(crate) struct FtileSwitchWithDiamonds {
    base: AbstractFtile,
    /// `FtileDimensionMemoize`: the geometry with a point out whatever the cases do.
    dimension_internal: OnceCell<FtileGeometry>,
    pub(super) tiles: Vec<Rc<dyn Ftile>>,
    in_: Option<SwimlaneId>,
    pub(super) diamond1: Rc<dyn Ftile>,
    pub(super) diamond2: Rc<dyn Ftile>,
    /// The labels of the arrows into the cases (`branch.getTextBlockPositive()`).
    pub(super) text_block_positives: Vec<Rc<dyn TextBlock>>,
    /// The labels of the arrows out of the cases (`branch.getTextBlockSpecial()`).
    pub(super) text_block_specials: Vec<Rc<dyn TextBlock>>,
    pub(super) mode: Mode,
    links: SwitchLinks,
    w13: f64,
    w9: f64,
    pub(super) arrow_color: Rainbow,
}

impl FtileSwitchWithDiamonds {
    #[allow(clippy::too_many_arguments, reason = "PlantUML's constructor")]
    pub(super) fn new(
        skin_param: Rc<SkinParam>,
        tiles: Vec<Rc<dyn Ftile>>,
        text_block_positives: Vec<Rc<dyn TextBlock>>,
        text_block_specials: Vec<Rc<dyn TextBlock>>,
        in_: Option<SwimlaneId>,
        diamond1: Rc<dyn Ftile>,
        diamond2: Rc<dyn Ftile>,
        string_bounder: &dyn StringBounder,
        arrow_color: Rainbow,
        links: SwitchLinks,
    ) -> Self {
        let first_right = tiles
            .first()
            .map_or(0.0, |tile| tile.calculate_dimension(string_bounder).get_right());
        let last_left = tiles
            .last()
            .map_or(0.0, |tile| tile.calculate_dimension(string_bounder).get_left());
        let w13 = diamond1.calculate_dimension(string_bounder).get_width() - first_right - last_left;
        let inner = tiles.get(1..tiles.len().saturating_sub(1)).unwrap_or_default();
        let w9 = inner.iter().fold(0.0, |result, tile| {
            result + tile.calculate_dimension(string_bounder).get_width()
        });
        let mode = if w13 > w9 {
            Mode::BigDiamond
        } else {
            Mode::SmallDiamond
        };
        Self {
            base: AbstractFtile::new(skin_param),
            dimension_internal: OnceCell::new(),
            tiles,
            in_,
            diamond1,
            diamond2,
            text_block_positives,
            text_block_specials,
            mode,
            links,
            w13,
            w9,
            arrow_color,
        }
    }

    fn get_ydelta1a(&self, string_bounder: &dyn StringBounder) -> f64 {
        match self.links {
            SwitchLinks::OneLink => 20.0,
            SwitchLinks::ManyLinks => {
                let mut max: f64 = 10.0;
                for label in &self.text_block_positives {
                    max = max.max(label.calculate_dimension(string_bounder).height);
                }
                if self.mode == Mode::BigDiamond {
                    let diamond_height = self
                        .diamond1
                        .calculate_dimension(string_bounder)
                        .get_height();
                    max += diamond_height / 2.0;
                }
                max + 10.0
            }
        }
    }

    fn get_ydelta1b() -> f64 {
        10.0
    }

    pub(super) fn calculate_dimension_internal(
        &self,
        string_bounder: &dyn StringBounder,
    ) -> FtileGeometry {
        *self
            .dimension_internal
            .get_or_init(|| self.calculate_dimension_internal_slow(string_bounder))
    }

    fn calculate_dimension_internal_slow(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let dim1 = self.diamond1.calculate_dimension(string_bounder);
        let dim2 = self.diamond2.calculate_dimension(string_bounder);
        let dim_nude = self.calculate_dimension_nude(string_bounder);
        let delta = self.get_ydelta1a(string_bounder) + Self::get_ydelta1b();
        match self.mode {
            Mode::BigDiamond => {
                let height = dim1.get_height() + dim_nude.get_height() + dim2.get_height() + delta;
                let (Some(first), Some(last)) = (self.tiles.first(), self.tiles.last()) else {
                    return dim_nude;
                };
                let tile0 = first.calculate_dimension(string_bounder);
                let width = tile0.get_width()
                    + SUPP15
                    + self.w13
                    + SUPP15
                    + last.calculate_dimension(string_bounder).get_width();
                FtileGeometry::with_out(
                    width,
                    height,
                    tile0.get_left() + SUPP15 + dim1.get_left(),
                    0.0,
                    height,
                )
            }
            Mode::SmallDiamond => dim1
                .append_bottom(dim_nude)
                .append_bottom(dim2)
                .add_dim(0.0, delta),
        }
    }

    /// The cases side by side (`FtileSwitchNude.calculateDimensionInternalSlow`).
    fn calculate_dimension_nude(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let mut result = XDimension2D::new(0.0, 0.0);
        for couple in &self.tiles {
            result = result.merge_lr(couple.calculate_dimension(string_bounder).dimension());
        }
        let result = result.delta(
            X_SEPARATION * (self.tiles.len() as f64 - 1.0),
            100.0,
        );
        FtileGeometry::new(result.width, result.height, result.width / 2.0, 0.0)
    }

    /// Where `tile`, a case, lies among the cases side by side (`getTranslateNude`).
    fn get_translate_nude(&self, tile: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        let mut x1 = 0.0;
        for candidate in &self.tiles {
            if same(candidate.as_ref(), tile) {
                return UTranslate::new(x1, 0.0);
            }
            x1 += candidate.calculate_dimension(string_bounder).get_width() + X_SEPARATION;
        }
        UTranslate::default()
    }

    /// Where `tile`, a case, is drawn (`getTranslateOf`).
    pub(super) fn get_translate_of(&self, tile: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        let main = self.get_translate_main(string_bounder);
        if self.mode == Mode::SmallDiamond {
            return self.get_translate_nude(tile, string_bounder).compose(main);
        }
        let count = self.tiles.len();
        let suppx = (self.w13 - self.w9) / (count as f64 - 1.0);
        let mut dx = 0.0;
        for candidate in &self.tiles[..count.saturating_sub(1)] {
            if same(candidate.as_ref(), tile) {
                return main.compose(UTranslate::new(dx, 0.0));
            }
            dx += candidate.calculate_dimension(string_bounder).get_width() + suppx;
        }
        let first_width = self
            .tiles
            .first()
            .map_or(0.0, |first| first.calculate_dimension(string_bounder).get_width());
        let dx9 = first_width + self.w13 + SUPP15 + SUPP15;
        main.compose(UTranslate::new(dx9, 0.0))
    }

    fn get_translate_main(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(
            0.0,
            dim_diamond1.get_height() + self.get_ydelta1a(string_bounder),
        )
    }

    pub(super) fn get_translate_diamond1(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_diamond1 = self.diamond1.calculate_dimension(string_bounder);
        UTranslate::new(dim_total.get_left() - dim_diamond1.get_left(), 0.0)
    }

    pub(super) fn get_translate_diamond2(&self, string_bounder: &dyn StringBounder) -> UTranslate {
        let dim_total = self.calculate_dimension_internal(string_bounder);
        let dim_diamond2 = self.diamond2.calculate_dimension(string_bounder);
        UTranslate::new(
            dim_total.get_left() - dim_diamond2.get_width() / 2.0,
            dim_total.get_height() - dim_diamond2.get_height(),
        )
    }
}

impl Swimable for FtileSwitchWithDiamonds {
    fn get_swimlanes(&self) -> SwimlaneSet {
        let mut result = SwimlaneSet::new();
        if let Some(in_) = self.in_ {
            result.insert(Some(in_));
        }
        for tile in &self.tiles {
            result.extend(tile.get_swimlanes());
        }
        result
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.in_
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.in_
    }
}

impl Ftile for FtileSwitchWithDiamonds {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_total = self.calculate_dimension_internal(string_bounder);
            if self
                .tiles
                .iter()
                .any(|tile| tile.calculate_dimension(string_bounder).has_point_out())
            {
                dim_total
            } else {
                dim_total.without_point_out()
            }
        })
    }

    /// Where a case lies among the cases, without the room above them, as in PlantUML.
    fn get_translate_for(&self, child: &dyn Ftile, string_bounder: &dyn StringBounder) -> UTranslate {
        self.get_translate_nude(child, string_bounder)
    }

    fn get_my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.tiles
            .iter()
            .chain([&self.diamond1, &self.diamond2])
            .cloned()
            .collect()
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        ug.apply(self.get_translate_diamond1(string_bounder))
            .draw(&self.diamond1);
        match self.mode {
            // PlantUML draws the cases themselves here, not through the layers.
            Mode::BigDiamond => {
                for tile in &self.tiles {
                    tile.draw_u(&ug.apply(self.get_translate_of(tile.as_ref(), string_bounder)));
                }
            }
            Mode::SmallDiamond => {
                let ug = ug.apply(self.get_translate_main(string_bounder));
                for tile in &self.tiles {
                    ug.apply(self.get_translate_nude(tile.as_ref(), string_bounder))
                        .draw(tile);
                }
            }
        }
        if self.calculate_dimension(string_bounder).has_point_out() {
            ug.apply(self.get_translate_diamond2(string_bounder))
                .draw(&self.diamond2);
        }
    }
}
