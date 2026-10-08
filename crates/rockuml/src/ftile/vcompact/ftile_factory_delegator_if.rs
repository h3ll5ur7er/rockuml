//! PlantUML's `FtileFactoryDelegatorIf`: lays out an `if` with `ConditionalBuilder`, or with
//! `FtileIfLongHorizontal` / `FtileIfLongVertical` when it has `elseif`s. It hands those builders
//! [`FtileFactoryDelegator::get_factory`], the chain inside it.

use std::rc::Rc;

use super::cond::ConditionalBuilder;
use super::{FtileIfLongHorizontal, FtileIfLongVertical};
use crate::diagram::activity3::{
    BranchFtile, Instructions, LinkRendering, PositionedNote, SwimlaneId,
};
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator};
use crate::klimt::url::Url;
use crate::stereo::Stereotype;
use crate::style::{PName, StyleBuilder, ValueReading};

pub(crate) struct FtileFactoryDelegatorIf {
    factory: Box<dyn FtileFactory>,
    /// `!pragma useVerticalIf on`: `elseif`s go down rather than across.
    use_vertical_if: bool,
}

impl FtileFactoryDelegatorIf {
    pub(crate) fn new(factory: Box<dyn FtileFactory>, use_vertical_if: bool) -> Self {
        Self {
            factory,
            use_vertical_if,
        }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorIf {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    fn create_if(
        &self,
        instructions: &Instructions,
        swimlane: Option<SwimlaneId>,
        thens: &[BranchFtile<'_>],
        else_branch: &BranchFtile<'_>,
        out_color: &LinkRendering,
        top_inlink_rendering: &LinkRendering,
        url: Option<&Url>,
        notes: &[PositionedNote],
        stereotype: Option<&Stereotype>,
        current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        let skin_param = FtileFactoryDelegator::skin_param(self);
        let condition_style = skin_param.get_condition_style();
        let condition_end_style = skin_param.get_condition_end_style();
        let branch0 = &thens[0];
        let style_arrow = self
            .get_default_style_definition_arrow()
            .get_merged_style(current_style_builder);
        let style_diamond = self
            .get_default_style_definition_diamond()
            .get_merged_style_with(current_style_builder, stereotype);
        let back_color = branch0
            .branch
            .get_color()
            .unwrap_or_else(|| style_diamond.value(PName::BackGroundColor).as_color());
        if thens.len() > 1 {
            if self.use_vertical_if {
                return FtileIfLongVertical::create(
                    swimlane,
                    &back_color,
                    self.get_factory(),
                    thens,
                    else_branch,
                    top_inlink_rendering,
                    &style_arrow,
                    &style_diamond,
                );
            }
            return FtileIfLongHorizontal::create(
                swimlane,
                &back_color,
                self.get_factory(),
                thens,
                else_branch,
                top_inlink_rendering,
                out_color,
                &style_arrow,
                &style_diamond,
            );
        }
        ConditionalBuilder::create(
            instructions,
            swimlane,
            back_color,
            self.get_factory(),
            condition_style,
            condition_end_style,
            branch0,
            else_branch,
            url,
            &style_arrow,
            style_diamond,
            notes,
        )
    }
}
