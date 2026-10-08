//! Lays out an `if` with one `then` and an `else` (PlantUML's `ConditionalBuilder`): `FtileIfDown` when a
//! branch is empty or a lone stop, `FtileIfWithLinks` otherwise.

use std::rc::Rc;

use super::FtileIfWithLinks;
use crate::color::HColor;
use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{BranchFtile, Instructions, SwimlaneId};
use crate::ftile::hexagon::HEXAGON_HALF_SIZE;
use crate::ftile::vcompact::{FtileIfDown, create0_or_empty};
use crate::ftile::vertical::{FtileDiamond, FtileDiamondInside, FtileDiamondSquare};
use crate::ftile::{
    Ftile, FtileEmpty, FtileFactory, FtileMinWidthCentered, FtileWithUrl, ftile_utils,
};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};
use crate::svek::{ConditionEndStyle, ConditionStyle};

pub(crate) struct ConditionalBuilder<'a> {
    swimlane: Option<SwimlaneId>,
    border_color: HColor,
    back_color: HColor,
    diamond_line_break: f64,
    label_line_break: f64,
    arrow_color: Rainbow,
    ftile_factory: &'a dyn FtileFactory,
    condition_style: ConditionStyle,
    condition_end_style: ConditionEndStyle,
    branch1: &'a BranchFtile<'a>,
    branch2: &'a BranchFtile<'a>,
    instructions: &'a Instructions,
    skin_param: Rc<SkinParam>,
    font_arrow: FontConfiguration,
    style_diamon_font: FontConfiguration,
    tile1: Rc<dyn Ftile>,
    tile2: Rc<dyn Ftile>,
    url: Option<&'a Url>,
    style_diamond: Style,
}

impl<'a> ConditionalBuilder<'a> {
    /// The tile of the `if`: `branch1` its `then`, `branch2` its `else`, built with `ftile_factory`, the
    /// factories inside the `if` delegator.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's create")]
    pub(crate) fn create(
        instructions: &'a Instructions,
        swimlane: Option<SwimlaneId>,
        back_color: HColor,
        ftile_factory: &'a dyn FtileFactory,
        condition_style: ConditionStyle,
        condition_end_style: ConditionEndStyle,
        branch1: &'a BranchFtile<'a>,
        branch2: &'a BranchFtile<'a>,
        url: Option<&'a Url>,
        style_arrow: &Style,
        style_diamond: Style,
    ) -> Rc<dyn Ftile> {
        let skin_param = Rc::clone(ftile_factory.skin_param());
        let builder = Self {
            swimlane,
            border_color: style_diamond.value(PName::LineColor).as_color(),
            back_color,
            diamond_line_break: style_diamond.wrap_width(),
            label_line_break: style_arrow.wrap_width(),
            arrow_color: Rainbow::build_from_style(style_arrow),
            ftile_factory,
            condition_style,
            condition_end_style,
            branch1,
            branch2,
            instructions,
            font_arrow: style_arrow.font_configuration(),
            style_diamon_font: style_diamond.font_configuration(),
            tile1: Rc::new(FtileMinWidthCentered::new(Rc::clone(&branch1.ftile), 30.0)),
            tile2: Rc::new(FtileMinWidthCentered::new(Rc::clone(&branch2.ftile), 30.0)),
            skin_param,
            url,
            style_diamond,
        };
        let empty_or_stop1 = builder.is_empty_or_only_single_stop_or_spot(branch1);
        let empty_or_stop2 = builder.is_empty_or_only_single_stop_or_spot(branch2);
        if empty_or_stop2 && !empty_or_stop1
            || branch1.is_empty() && builder.is_only_single_stop_or_spot(branch2)
        {
            return builder.create_down(branch1, branch2);
        }
        if empty_or_stop1 && !empty_or_stop2
            || branch2.is_empty() && builder.is_only_single_stop_or_spot(branch1)
        {
            return builder.create_down(branch2, branch1);
        }
        builder.create_with_links()
    }

    fn is_only_single_stop_or_spot(&self, branch: &BranchFtile<'_>) -> bool {
        branch.is_only_single_stop_or_spot(self.instructions)
    }

    fn is_empty_or_only_single_stop_or_spot(&self, branch: &BranchFtile<'_>) -> bool {
        branch.is_empty() || self.is_only_single_stop_or_spot(branch)
    }

    fn string_bounder(&self) -> &dyn StringBounder {
        self.ftile_factory.get_string_bounder()
    }

    /// `branch1` below the diamond, `branch2` going round it or, when it is a lone stop, beside it.
    /// `branch1` is never empty here: of an empty branch and a lone stop, the stop is `branch2`.
    fn create_down(&self, branch1: &BranchFtile<'_>, branch2: &BranchFtile<'_>) -> Rc<dyn Ftile> {
        let tile1: Rc<dyn Ftile> =
            Rc::new(FtileMinWidthCentered::new(Rc::clone(&branch1.ftile), 30.0));
        let tb1 = self.get_label_positive(branch1);
        let tb2 = self.get_label_positive(branch2);
        let diamond1 = self.get_shape1(false, tb1, tb2);
        let diamond2 = self.get_shape2(branch1, branch2, true);
        let optional_stop = self
            .is_only_single_stop_or_spot(branch2)
            .then(|| Rc::clone(&branch2.ftile));
        FtileIfDown::create(
            diamond1,
            diamond2,
            self.swimlane,
            ftile_utils::add_horizontal_margin(tile1, 10.0, 10.0),
            &self.arrow_color,
            self.condition_end_style,
            self.ftile_factory,
            optional_stop,
            &branch2.branch.get_out(),
        )
    }

    fn create_with_links(&self) -> Rc<dyn Ftile> {
        let string_bounder = self.string_bounder();
        let mut diamond1 = self.get_diamond1(true);
        if let Some(url) = self.url {
            diamond1 = Rc::new(FtileWithUrl::new(diamond1, url.clone()));
        }
        let diamond2 = self.get_shape2(self.branch1, self.branch2, false);
        let tmp1 = ftile_utils::add_horizontal_margin(Rc::clone(&self.tile1), 10.0, 10.0);
        let tmp2 = ftile_utils::add_horizontal_margin(Rc::clone(&self.tile2), 10.0, 10.0);
        let ftile = Rc::new(FtileIfWithLinks::new(
            Rc::clone(&self.skin_param),
            diamond1,
            tmp1,
            tmp2,
            diamond2,
            self.swimlane,
            self.arrow_color.clone(),
            self.condition_end_style,
        ));
        let label1 = self
            .get_label_positive(self.branch1)
            .calculate_dimension(string_bounder);
        let label2 = self
            .get_label_positive(self.branch2)
            .calculate_dimension(string_bounder);
        let diff1 = ftile.compute_margin_need_for_branch_labe1(string_bounder, label1);
        let diff2 = ftile.compute_margin_need_for_branch_labe2(string_bounder, label2);
        let supp_height =
            ftile.compute_vertical_margin_need_for_branchs(string_bounder, label1, label2);
        let result = ftile.add_links(self.branch1, self.branch2, string_bounder);
        let result = ftile_utils::add_horizontal_margin(result, diff1, diff2);
        ftile_utils::add_vertical_margin(result, supp_height, 0.0)
    }

    fn get_diamond1(&self, east_west: bool) -> Rc<dyn Ftile> {
        self.get_shape1(
            east_west,
            self.get_label_positive(self.branch1),
            self.get_label_positive(self.branch2),
        )
    }

    /// The condition's diamond, the labels of the branches beside it (`east_west`) or below and beside it.
    fn get_shape1(
        &self,
        east_west: bool,
        tb1: Rc<dyn TextBlock>,
        tb2: Rc<dyn TextBlock>,
    ) -> Rc<dyn Ftile> {
        let tb_test = self.label_test_block();
        let skin_param = Rc::clone(&self.skin_param);
        let (back_color, border_color) = (self.back_color.clone(), self.border_color.clone());
        match self.condition_style {
            ConditionStyle::InsideHexagon => {
                let diamond = FtileDiamondInside::new(
                    tb_test,
                    skin_param,
                    back_color,
                    border_color,
                    self.swimlane,
                );
                if east_west {
                    Rc::new(diamond.with_west_and_east(tb1, tb2))
                } else {
                    Rc::new(diamond.with_south(tb1).with_east(tb2))
                }
            }
            ConditionStyle::EmptyDiamond => {
                let diamond =
                    FtileDiamond::new(skin_param, back_color, border_color, self.swimlane)
                        .with_north(tb_test);
                if east_west {
                    Rc::new(diamond.with_west_and_east(tb1, tb2))
                } else {
                    Rc::new(diamond.with_south(tb1).with_east(tb2))
                }
            }
            ConditionStyle::InsideDiamond => {
                let diamond = FtileDiamondSquare::new(
                    tb_test,
                    skin_param,
                    back_color,
                    border_color,
                    self.swimlane,
                );
                if east_west {
                    Rc::new(diamond.with_west_and_east(tb1, tb2))
                } else {
                    Rc::new(diamond.with_south(tb1).with_east(tb2))
                }
            }
        }
    }

    /// The condition, in the diamond style's font and alignment.
    fn label_test_block(&self) -> Rc<dyn TextBlock> {
        let horizontal_alignment = self
            .style_diamond
            .horizontal_alignment()
            .unwrap_or(HorizontalAlignment::Left);
        let label_test = self.branch1.branch.label_test.clone().unwrap_or_default();
        let sheet = CreoleParser::with_mode(
            self.style_diamon_font.clone(),
            horizontal_alignment,
            CreoleMode::Full,
            self.skin_param.as_ref(),
        )
        .create_display_sheet(&label_test, &self.style_diamon_font);
        Rc::new(SheetBlock2::new(
            SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none())
                .wrapped_at(self.diamond_line_break),
        ))
    }

    /// The label of the arrow into a branch (`getLabelPositive`).
    fn get_label_positive(&self, branch: &BranchFtile<'_>) -> Rc<dyn TextBlock> {
        self.create_arrow_label(
            branch.get_display_positive().as_ref(),
            self.label_line_break,
        )
    }

    /// A label in the arrow font.
    fn create_arrow_label(&self, display: Option<&Display>, max_width: f64) -> Rc<dyn TextBlock> {
        create0_or_empty(
            display,
            &self.font_arrow,
            HorizontalAlignment::Left,
            &self.skin_param,
            max_width,
            CreoleMode::SimpleLine,
        )
    }

    /// What ends the conditional: a diamond with the labels of the arrows out of the branches, a point
    /// where only one branch goes on, or room for the line of `conditionEndStyle hline`.
    fn get_shape2(
        &self,
        branch1: &BranchFtile<'_>,
        branch2: &BranchFtile<'_>,
        use_north: bool,
    ) -> Rc<dyn Ftile> {
        if self.condition_end_style == ConditionEndStyle::Hline {
            return Rc::new(FtileEmpty::with_size(
                Rc::clone(&self.skin_param),
                0.0,
                HEXAGON_HALF_SIZE,
                self.swimlane,
            ));
        }
        if !self.has_two_branches() {
            return Rc::new(FtileEmpty::with_size(
                Rc::clone(&self.skin_param),
                0.0,
                HEXAGON_HALF_SIZE / 2.0,
                self.swimlane,
            ));
        }
        let tbout1 =
            self.create_arrow_label(branch1.ftile.get_out_link_rendering().display.as_ref(), 0.0);
        let tbout2 =
            self.create_arrow_label(branch2.ftile.get_out_link_rendering().display.as_ref(), 0.0);
        let color = branch2
            .branch
            .get_color()
            .unwrap_or_else(|| self.back_color.clone());
        let diamond = FtileDiamond::new(
            Rc::clone(&self.skin_param),
            color,
            self.border_color.clone(),
            self.swimlane,
        );
        let diamond = if use_north {
            diamond.with_north(tbout1)
        } else {
            diamond.with_west(tbout1)
        };
        Rc::new(diamond.with_east(tbout2))
    }

    fn has_two_branches(&self) -> bool {
        let string_bounder = self.string_bounder();
        self.tile1
            .calculate_dimension(string_bounder)
            .has_point_out()
            && self
                .tile2
                .calculate_dimension(string_bounder)
                .has_point_out()
    }
}
