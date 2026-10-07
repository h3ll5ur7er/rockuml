//! What builds the tiles of instructions (PlantUML's `FtileFactory`).

use std::rc::Rc;

use super::{BoxStyle, Ftile};
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::diagram::activity3::{LinkRendering, SwimlaneId};
use crate::klimt::font::StringBounder;
use crate::klimt::url::Url;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::StyleBuilder;

/// Implemented by `VCompactFactory`, the innermost factory, and through [`super::FtileFactoryDelegator`]
/// by the delegators around it. Instructions call the outermost one.
///
/// The methods taking the model's types (`createIf`, `createSwitch`, `createWhile`, `repeat`,
/// `createParallel`, `addNote`, `createGroup`) join once the model has them.
pub(crate) trait FtileFactory {
    fn get_string_bounder(&self) -> &dyn StringBounder;

    /// Shared with the tiles built.
    fn skin_param(&self) -> &Rc<SkinParam>;

    fn start(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile>;

    fn stop(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile>;

    fn end(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile>;

    fn spot(
        &self,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        color: Option<HColor>,
    ) -> Rc<dyn Ftile>;

    /// `style_builder` holds the styles in force where the activity was declared.
    fn activity(
        &self,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<&Stereotype>,
        style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile>;

    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile>;

    /// The tile with the arrow into it drawn as `link_rendering` says.
    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile>;

    /// The tile with the arrow out of it drawn as `link_rendering` says.
    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile>;

    /// `tile1` above `tile2`, joined by an arrow.
    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile>;
}
