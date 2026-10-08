//! PlantUML's `FtileFactoryDelegatorAssembly`: stacks two tiles with room between them for the arrow and
//! its label, and draws the arrow.

use std::rc::Rc;

use super::ConnectionVerticalDown;
use crate::ftile::{Ftile, FtileFactory, FtileFactoryDelegator, FtileMargedRight, ftile_utils};

pub(crate) struct FtileFactoryDelegatorAssembly {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorAssembly {
    pub(crate) fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactoryDelegator for FtileFactoryDelegatorAssembly {
    fn get_factory(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        let mut height = 35.0;
        let text_block =
            self.get_text_block(self.get_in_link_rendering_display(tile2.as_ref()).as_ref());
        let string_bounder = FtileFactoryDelegator::get_string_bounder(self);
        if let Some(text_block) = &text_block {
            height += text_block.calculate_dimension(string_bounder).height;
        }
        let tile1_and_space = ftile_utils::add_bottom(tile1.clone(), height);
        let result = self
            .get_factory()
            .assembly(tile1_and_space.clone(), tile2.clone());
        let geo = tile1.calculate_dimension(string_bounder);
        if !geo.has_point_out() {
            return result;
        }
        let translate1 = result.get_translate_for(tile1_and_space.as_ref(), string_bounder);
        let p1 = geo.translate(translate1).get_point_out();
        let translate2 = result.get_translate_for(tile2.as_ref(), string_bounder);
        let p2 = tile2
            .calculate_dimension(string_bounder)
            .translate(translate2)
            .get_point_in();
        let color = self.get_in_link_rendering_color(tile2.as_ref());
        let has_label = text_block.is_some();
        let connection = Rc::new(ConnectionVerticalDown::new(
            tile1, tile2, p1, p2, color, text_block,
        ));
        let result = ftile_utils::add_connection(result, connection.clone());
        if has_label {
            let width = result.calculate_dimension(string_bounder).get_width();
            let max_x = connection.get_max_x(string_bounder);
            if width < max_x {
                return Rc::new(FtileMargedRight::new(result, max_x));
            }
        }
        result
    }
}
