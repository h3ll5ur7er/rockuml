//! What builds the tiles of instructions (PlantUML's `FtileFactory`).

use std::rc::Rc;

use super::{BoxStyle, Ftile};
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::decoration::symbol::USymbol;
use crate::diagram::activity3::{
    BranchFtile, ForkStyle, InstructionId, Instructions, LinkRendering, PositionedNote, SwimlaneId,
};
use crate::klimt::VerticalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::url::Url;
use crate::skin::SkinParam;
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{Style, StyleBuilder};

/// Implemented by `VCompactFactory`, the innermost factory, and through [`super::FtileFactoryDelegator`]
/// by the delegators around it. Instructions call the outermost one.
///
/// PlantUML's `Display.NULL` is `None`. Where PlantUML hands a factory instructions or branches, it gets
/// the instruction arena along with their ids, and each branch with the tile already built for it.
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

    /// `ftile` with `notes` beside it, or the first note alone without a tile.
    fn add_note(
        &self,
        ftile: Option<Rc<dyn Ftile>>,
        swimlane: Option<SwimlaneId>,
        notes: &[PositionedNote],
        vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile>;

    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile>;

    /// The tile with the arrow into it drawn as `link_rendering` says.
    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile>;

    /// The tile with the arrow out of it drawn as `link_rendering` says.
    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile>;

    /// `tile1` above `tile2`, joined by an arrow.
    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile>;

    /// `no_out` is PlantUML's `isLastOfTheParent()`.
    #[allow(clippy::too_many_arguments, reason = "PlantUML's FtileFactory.repeat")]
    fn repeat(
        &self,
        stereotype: &Stereogroup,
        stereotype2: &Stereogroup,
        box_style_in: BoxStyle,
        swimlane: Option<SwimlaneId>,
        swimlane_out: Option<SwimlaneId>,
        start_label: Option<&Display>,
        repeat: Rc<dyn Ftile>,
        test: Option<&Display>,
        yes: Option<&Display>,
        out: Option<&Display>,
        backward: Option<Rc<dyn Ftile>>,
        no_out: bool,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
        current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile>;

    /// `special_out` is the `stop` or `end` right after `endwhile`, which `FtileWhile` builds itself.
    #[allow(
        clippy::too_many_arguments,
        reason = "PlantUML's FtileFactory.createWhile"
    )]
    fn create_while(
        &self,
        instructions: &Instructions,
        out_color: &LinkRendering,
        swimlane: Option<SwimlaneId>,
        while_block: Rc<dyn Ftile>,
        test: &Display,
        yes: Option<&Display>,
        color: Option<HColor>,
        special_out: Option<InstructionId>,
        backward: Option<Rc<dyn Ftile>>,
        incoming1: &LinkRendering,
        incoming2: &LinkRendering,
        current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile>;

    #[allow(
        clippy::too_many_arguments,
        reason = "PlantUML's FtileFactory.createIf"
    )]
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
    ) -> Rc<dyn Ftile>;

    #[allow(
        clippy::too_many_arguments,
        reason = "PlantUML's FtileFactory.createSwitch"
    )]
    fn create_switch(
        &self,
        instructions: &Instructions,
        swimlane: Option<SwimlaneId>,
        branches: &[BranchFtile<'_>],
        after_endwhile: &LinkRendering,
        top_inlink_rendering: &LinkRendering,
        label_test: Option<&Display>,
        colors: &Colors,
        end_colors: &Colors,
    ) -> Rc<dyn Ftile>;

    /// `label` is the join specification of a fork, like `{or}`.
    fn create_parallel(
        &self,
        all: Vec<Rc<dyn Ftile>>,
        style: ForkStyle,
        label: Option<&str>,
        swimlane_in: Option<SwimlaneId>,
        swimlane_out: Option<SwimlaneId>,
        colors: &Colors,
    ) -> Rc<dyn Ftile>;

    fn create_group(
        &self,
        list: Rc<dyn Ftile>,
        name: &Display,
        back_color: Option<HColor>,
        note: Option<&PositionedNote>,
        type_: USymbol,
        style: &Style,
    ) -> Rc<dyn Ftile>;
}
