//! PlantUML's `FtileFactoryDelegatorCreateParallel`: joins the flows of a fork, merge or split, which the
//! factories inside lay side by side, with bars and arrows.

use std::rc::Rc;

use super::abstract_parallel_ftiles_builder::ParallelFtilesBuilder;
use super::parallel_builder_fork::ParallelBuilderFork;
use super::parallel_builder_merge::ParallelBuilderMerge;
use super::parallel_builder_split::ParallelBuilderSplit;
use crate::color::Colors;
use crate::diagram::activity3::{ForkStyle, SwimlaneId};
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator};

pub(crate) struct FtileFactoryDelegatorCreateParallel {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorCreateParallel {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorCreateParallel {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    fn create_parallel(
        &self,
        all: Vec<Rc<dyn Ftile>>,
        style: ForkStyle,
        label: Option<&str>,
        swimlane_in: Option<SwimlaneId>,
        swimlane_out: Option<SwimlaneId>,
        colors: &Colors,
    ) -> Rc<dyn Ftile> {
        let skin_param = FtileFactoryDelegator::skin_param(self).clone();
        let string_bounder = FtileFactoryDelegator::get_string_bounder(self);
        let builder: Box<dyn ParallelFtilesBuilder> = match style {
            ForkStyle::Split => Box::new(ParallelBuilderSplit::new(
                skin_param,
                string_bounder,
                &all,
                colors.clone(),
            )),
            ForkStyle::Merge => Box::new(ParallelBuilderMerge::new(
                skin_param,
                string_bounder,
                &all,
                colors.clone(),
            )),
            ForkStyle::Fork => Box::new(ParallelBuilderFork::new(
                skin_param,
                string_bounder,
                label,
                swimlane_in,
                swimlane_out,
                &all,
                colors.clone(),
            )),
        };
        let inner = self.get_factory().create_parallel(
            builder.list99().to_vec(),
            style,
            label,
            swimlane_in,
            swimlane_out,
            colors,
        );
        builder.build(&inner)
    }
}
