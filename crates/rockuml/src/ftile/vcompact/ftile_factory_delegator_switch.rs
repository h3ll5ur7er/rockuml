//! PlantUML's `FtileFactoryDelegatorSwitch`: lays out a `switch`, its cases side by side between two
//! diamonds, with the arrows of `FtileSwitchWithOneLink` or `FtileSwitchWithManyLinks`
//! (`createWithLinks`).

use std::rc::Rc;

use super::cond::{ftile_switch_with_many_links, ftile_switch_with_one_link};
use crate::color::Colors;
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{BranchFtile, Instructions, LinkRendering, SwimlaneId};
use crate::ftile::vertical::{
    FtileDecorateInLabel, FtileDecorateOutLabel, FtileDiamondInside, empty_label,
};
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, Style, ValueReading};

pub(crate) struct FtileFactoryDelegatorSwitch {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorSwitch {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }

    fn diamond_style(&self) -> Style {
        self.get_default_style_definition_diamond()
            .get_merged_style(&FtileFactoryDelegator::skin_param(self).current_style_builder())
    }

    /// The diamond the cases leave, the switch's test inside.
    fn get_diamond1(
        &self,
        swimlane: Option<SwimlaneId>,
        branch0: &BranchFtile<'_>,
        test: Option<&Display>,
        colors: &Colors,
    ) -> Rc<dyn Ftile> {
        let skin_param = FtileFactoryDelegator::skin_param(self);
        let style = self.diamond_style();
        let border_color = style.value(PName::LineColor).as_color();
        let back_color = branch0
            .branch
            .get_color()
            .unwrap_or_else(|| colors.get_color_of(&style, PName::BackGroundColor));
        let tb_test: Rc<dyn TextBlock> = match test {
            Some(test) if !test.is_white() => Rc::new(
                test.create0(
                    &style.font_configuration(),
                    branch0
                        .ftile
                        .skin_param()
                        .get_default_text_alignment(HorizontalAlignment::Left),
                    branch0.ftile.skin_param(),
                    style.wrap_width(),
                    CreoleMode::Full,
                ),
            ),
            _ => empty_label(),
        };
        Rc::new(FtileDiamondInside::new(
            tb_test,
            Rc::clone(skin_param),
            back_color,
            border_color,
            swimlane,
        ))
    }

    /// The diamond the cases join.
    fn get_diamond2(
        &self,
        swimlane: Option<SwimlaneId>,
        branch0: &BranchFtile<'_>,
        end_colors: &Colors,
    ) -> Rc<dyn Ftile> {
        let style = self.diamond_style();
        let border_color = style.value(PName::LineColor).as_color();
        let back_color = branch0
            .branch
            .get_color()
            .unwrap_or_else(|| end_colors.get_color_of(&style, PName::BackGroundColor));
        Rc::new(FtileDiamondInside::new(
            empty_label(),
            Rc::clone(FtileFactoryDelegator::skin_param(self)),
            back_color,
            border_color,
            swimlane,
        ))
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorSwitch {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    fn create_switch(
        &self,
        _instructions: &Instructions,
        swimlane: Option<SwimlaneId>,
        branches: &[BranchFtile<'_>],
        _after_endwhile: &LinkRendering,
        _top_inlink_rendering: &LinkRendering,
        label_test: Option<&Display>,
        colors: &Colors,
        end_colors: &Colors,
    ) -> Rc<dyn Ftile> {
        let string_bounder = FtileFactoryDelegator::get_string_bounder(self);
        let branch0 = &branches[0];
        let diamond1 = self.get_diamond1(swimlane, branch0, label_test, colors);
        let diamond2 = self.get_diamond2(swimlane, branch0, end_colors);
        let mut ftiles: Vec<Rc<dyn Ftile>> = Vec::new();
        let mut positives = Vec::new();
        let mut specials = Vec::new();
        for branch in branches {
            let positive = branch.get_text_block_positive();
            let special = branch.get_text_block_special();
            let dim_label_in = positive.calculate_dimension(string_bounder);
            let dim_label_out = special.calculate_dimension(string_bounder);
            ftiles.push(Rc::new(FtileDecorateOutLabel::new(
                Rc::new(FtileDecorateInLabel::new(
                    Rc::clone(&branch.ftile),
                    dim_label_in,
                )),
                dim_label_out,
            )));
            positives.push(positive);
            specials.push(special);
        }
        let arrow_color = Rainbow::build_from_style(&self.arrow_style());
        let skin_param = Rc::clone(FtileFactoryDelegator::skin_param(self));
        if let ([tile], [positive], [special]) =
            (ftiles.as_slice(), positives.as_slice(), specials.as_slice())
        {
            return ftile_switch_with_one_link::create(
                skin_param,
                Rc::clone(tile),
                Rc::clone(positive),
                Rc::clone(special),
                swimlane,
                diamond1,
                diamond2,
                string_bounder,
                arrow_color,
            );
        }
        ftile_switch_with_many_links::create(
            skin_param,
            ftiles,
            positives,
            specials,
            swimlane,
            diamond1,
            diamond2,
            string_bounder,
            arrow_color,
        )
    }
}
