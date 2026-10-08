//! A tile that follows a link when clicked (PlantUML's `FtileWithUrl`).

use std::rc::Rc;

use super::Ftile;
use super::vertical::FtileDecorate;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;

pub(crate) struct FtileWithUrl {
    ftile: Rc<dyn Ftile>,
    url: Url,
}

impl FtileWithUrl {
    pub(crate) fn new(ftile: Rc<dyn Ftile>, url: Url) -> Self {
        Self { ftile, url }
    }
}

impl FtileDecorate for FtileWithUrl {
    fn get_ftile_delegated(&self) -> &Rc<dyn Ftile> {
        &self.ftile
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.start_url(&self.url);
        self.ftile.draw_u(ug);
        ug.close_url();
    }
}
