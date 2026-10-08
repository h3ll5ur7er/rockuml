//! PlantUML's `FtileFactoryDelegatorCreateGroup`: groups, partitions, packages, rectangles and cards.

use std::rc::Rc;

use super::{FtileGroup, FtileWithNotes};
use crate::color::HColor;
use crate::creole::Display;
use crate::decoration::symbol::USymbol;
use crate::diagram::activity3::PositionedNote;
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator};
use crate::klimt::VerticalAlignment;
use crate::style::Style;

pub(crate) struct FtileFactoryDelegatorCreateGroup {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorCreateGroup {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorCreateGroup {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    fn create_group(
        &self,
        list: Rc<dyn Ftile>,
        name: &Display,
        back_color: Option<HColor>,
        note: Option<&PositionedNote>,
        type_: USymbol,
        style: &Style,
    ) -> Rc<dyn Ftile> {
        let result: Rc<dyn Ftile> = Rc::new(FtileGroup::new(
            list,
            name,
            back_color,
            FtileFactoryDelegator::skin_param(self),
            type_,
            style,
        ));
        match note {
            Some(note) => Rc::new(FtileWithNotes::new(
                result,
                std::slice::from_ref(note),
                VerticalAlignment::Center,
            )),
            None => result,
        }
    }
}
