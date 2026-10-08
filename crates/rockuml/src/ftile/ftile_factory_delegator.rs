//! A factory wrapping another, overriding some methods and passing the others on (PlantUML's
//! `FtileFactoryDelegator`), with the helpers its subclasses share.

use std::rc::Rc;

use super::{BoxStyle, Ftile, FtileFactory};
use crate::color::{Colors, HColor};
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::decoration::symbol::USymbol;
use crate::diagram::activity3::{
    BranchFtile, ForkStyle, InstructionId, Instructions, LinkRendering, PositionedNote, SwimlaneId,
};
use crate::klimt::font::StringBounder;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::skin::SkinParam;
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{SName, Style, StyleBuilder, StyleSignature};

/// A delegator implements [`Self::get_factory`] and overrides what it changes; it is an [`FtileFactory`]
/// through the blanket implementation below. Overrides reach the wrapped factory (PlantUML's
/// `super.method(...)`) with `self.get_factory().method(...)`.
pub(crate) trait FtileFactoryDelegator {
    /// The factory wrapped, which builds what this one does not change (`getFactory`). Builders that
    /// PlantUML hands `getFactory()` get it too, so they skip this delegator and those outside it.
    fn get_factory(&self) -> &dyn FtileFactory;

    fn get_string_bounder(&self) -> &dyn StringBounder {
        self.get_factory().get_string_bounder()
    }

    fn skin_param(&self) -> &Rc<SkinParam> {
        self.get_factory().skin_param()
    }

    fn start(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        self.get_factory().start(swimlane, colors)
    }

    fn stop(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        self.get_factory().stop(swimlane, colors)
    }

    fn end(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        self.get_factory().end(swimlane, colors)
    }

    fn spot(
        &self,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        color: Option<HColor>,
    ) -> Rc<dyn Ftile> {
        self.get_factory().spot(swimlane, spot, color)
    }

    fn activity(
        &self,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<&Stereotype>,
        style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        self.get_factory()
            .activity(label, swimlane, style, colors, stereotype, style_builder)
    }

    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile> {
        self.get_factory().add_url(ftile, url)
    }

    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        self.get_factory().decorate_in(ftile, link_rendering)
    }

    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        self.get_factory().decorate_out(ftile, link_rendering)
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        self.get_factory().assembly(tile1, tile2)
    }

    fn add_note(
        &self,
        ftile: Option<Rc<dyn Ftile>>,
        swimlane: Option<SwimlaneId>,
        notes: &[PositionedNote],
        vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile> {
        self.get_factory()
            .add_note(ftile, swimlane, notes, vertical_alignment)
    }

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
    ) -> Rc<dyn Ftile> {
        self.get_factory().repeat(
            stereotype,
            stereotype2,
            box_style_in,
            swimlane,
            swimlane_out,
            start_label,
            repeat,
            test,
            yes,
            out,
            backward,
            no_out,
            incoming1,
            incoming2,
            current_style_builder,
        )
    }

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
    ) -> Rc<dyn Ftile> {
        self.get_factory().create_while(
            instructions,
            out_color,
            swimlane,
            while_block,
            test,
            yes,
            color,
            special_out,
            backward,
            incoming1,
            incoming2,
            current_style_builder,
        )
    }

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
    ) -> Rc<dyn Ftile> {
        self.get_factory().create_if(
            instructions,
            swimlane,
            thens,
            else_branch,
            out_color,
            top_inlink_rendering,
            url,
            notes,
            stereotype,
            current_style_builder,
        )
    }

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
        end_colors: Option<&Colors>,
    ) -> Rc<dyn Ftile> {
        self.get_factory().create_switch(
            instructions,
            swimlane,
            branches,
            after_endwhile,
            top_inlink_rendering,
            label_test,
            colors,
            end_colors,
        )
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
        self.get_factory()
            .create_parallel(all, style, label, swimlane_in, swimlane_out, colors)
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
        self.get_factory()
            .create_group(list, name, back_color, note, type_, style)
    }

    fn get_default_style_definition_activity(&self) -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
        ])
    }

    fn get_default_style_definition_diamond(&self) -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
            SName::Diamond,
        ])
    }

    fn get_default_style_definition_arrow(&self) -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
    }

    /// The style of activity arrows.
    fn arrow_style(&self) -> Style {
        self.skin_param()
            .merged_style(&self.get_default_style_definition_arrow())
            .expect("the skin styles activity arrows")
    }

    /// The colours of the arrow into `tile`, or else those of the arrow style (`getInLinkRenderingColor`).
    fn get_in_link_rendering_color(&self, tile: &dyn Ftile) -> Rainbow {
        let color = tile.get_in_link_rendering().rainbow;
        if color.size() == 0 {
            return Rainbow::build_from_style(&self.arrow_style());
        }
        color
    }

    /// An arrow label in the arrow style's font; none without a label (`getTextBlock`).
    fn get_text_block(&self, display: Option<&Display>) -> Option<Rc<dyn TextBlock>> {
        let display = display?;
        let font_configuration = self.arrow_style().font_configuration();
        Some(Rc::new(display.create0(
            &font_configuration,
            HorizontalAlignment::Left,
            self.skin_param().as_ref(),
            0.0,
            CreoleMode::SimpleLine,
        )))
    }

    /// The label of the arrow into `tile` (`getInLinkRenderingDisplay`).
    fn get_in_link_rendering_display(&self, tile: &dyn Ftile) -> Option<Display> {
        tile.get_in_link_rendering().display
    }
}

impl<T: FtileFactoryDelegator> FtileFactory for T {
    fn get_string_bounder(&self) -> &dyn StringBounder {
        FtileFactoryDelegator::get_string_bounder(self)
    }

    fn skin_param(&self) -> &Rc<SkinParam> {
        FtileFactoryDelegator::skin_param(self)
    }

    fn start(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::start(self, swimlane, colors)
    }

    fn stop(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::stop(self, swimlane, colors)
    }

    fn end(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::end(self, swimlane, colors)
    }

    fn spot(
        &self,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        color: Option<HColor>,
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::spot(self, swimlane, spot, color)
    }

    fn activity(
        &self,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<&Stereotype>,
        style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::activity(
            self,
            label,
            swimlane,
            style,
            colors,
            stereotype,
            style_builder,
        )
    }

    fn add_url(&self, ftile: Rc<dyn Ftile>, url: &Url) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::add_url(self, ftile, url)
    }

    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::decorate_in(self, ftile, link_rendering)
    }

    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::decorate_out(self, ftile, link_rendering)
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::assembly(self, tile1, tile2)
    }

    fn add_note(
        &self,
        ftile: Option<Rc<dyn Ftile>>,
        swimlane: Option<SwimlaneId>,
        notes: &[PositionedNote],
        vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::add_note(self, ftile, swimlane, notes, vertical_alignment)
    }

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
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::repeat(
            self,
            stereotype,
            stereotype2,
            box_style_in,
            swimlane,
            swimlane_out,
            start_label,
            repeat,
            test,
            yes,
            out,
            backward,
            no_out,
            incoming1,
            incoming2,
            current_style_builder,
        )
    }

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
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::create_while(
            self,
            instructions,
            out_color,
            swimlane,
            while_block,
            test,
            yes,
            color,
            special_out,
            backward,
            incoming1,
            incoming2,
            current_style_builder,
        )
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
        FtileFactoryDelegator::create_if(
            self,
            instructions,
            swimlane,
            thens,
            else_branch,
            out_color,
            top_inlink_rendering,
            url,
            notes,
            stereotype,
            current_style_builder,
        )
    }

    fn create_switch(
        &self,
        instructions: &Instructions,
        swimlane: Option<SwimlaneId>,
        branches: &[BranchFtile<'_>],
        after_endwhile: &LinkRendering,
        top_inlink_rendering: &LinkRendering,
        label_test: Option<&Display>,
        colors: &Colors,
        end_colors: Option<&Colors>,
    ) -> Rc<dyn Ftile> {
        FtileFactoryDelegator::create_switch(
            self,
            instructions,
            swimlane,
            branches,
            after_endwhile,
            top_inlink_rendering,
            label_test,
            colors,
            end_colors,
        )
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
        FtileFactoryDelegator::create_parallel(
            self,
            all,
            style,
            label,
            swimlane_in,
            swimlane_out,
            colors,
        )
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
        FtileFactoryDelegator::create_group(self, list, name, back_color, note, type_, style)
    }
}
