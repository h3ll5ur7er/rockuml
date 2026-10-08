//! The innermost factory (PlantUML's `VCompactFactory`): it builds the tiles of single instructions and
//! stacks them; what it does for compound instructions, the delegators around it override.

use std::rc::Rc;

use super::FtileForkInner;
use crate::color::{Colors, HColor};
use crate::creole::Display;
use crate::decoration::symbol::USymbol;
use crate::diagram::activity3::{
    BranchFtile, ForkStyle, InstructionId, Instructions, LinkRendering, PositionedNote, SwimlaneId,
};
use crate::ftile::vertical::{
    FtileBox, FtileCircleEndCross, FtileCircleSpot, FtileCircleStart, FtileCircleStop,
    FtileDecorateIn, FtileDecorateOut,
};
use crate::ftile::{BoxStyle, Ftile, FtileAssemblySimple, FtileEmpty, FtileFactory};
use crate::klimt::VerticalAlignment;
use crate::klimt::font::StringBounder;
use crate::klimt::url::Url;
use crate::skin::SkinParam;
use crate::stereo::{Stereogroup, Stereotype};
use crate::style::{SName, Style, StyleBuilder, StyleSignature};

pub(crate) struct VCompactFactory {
    skin_param: Rc<SkinParam>,
    string_bounder: Rc<dyn StringBounder>,
}

impl VCompactFactory {
    pub(crate) fn new(skin_param: Rc<SkinParam>, string_bounder: Rc<dyn StringBounder>) -> Self {
        Self {
            skin_param,
            string_bounder,
        }
    }

    /// The style of the circle `name` names, as the diagram's styles give it.
    fn circle_style(&self, name: SName) -> Style {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Circle,
            name,
        ])
        .get_merged_style(&self.skin_param.current_style_builder())
    }
}

impl FtileFactory for VCompactFactory {
    fn get_string_bounder(&self) -> &dyn StringBounder {
        self.string_bounder.as_ref()
    }

    fn skin_param(&self) -> &Rc<SkinParam> {
        &self.skin_param
    }

    fn start(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        let style = self.circle_style(SName::Start);
        Rc::new(FtileCircleStart::new(
            self.skin_param.clone(),
            swimlane,
            &style,
            colors,
        ))
    }

    fn stop(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        let style = self.circle_style(SName::Stop);
        Rc::new(FtileCircleStop::new(
            self.skin_param.clone(),
            swimlane,
            &style,
            colors,
        ))
    }

    fn end(&self, swimlane: Option<SwimlaneId>, colors: &Colors) -> Rc<dyn Ftile> {
        let style = self.circle_style(SName::End);
        Rc::new(FtileCircleEndCross::new(
            self.skin_param.clone(),
            swimlane,
            &style,
            colors,
        ))
    }

    fn spot(
        &self,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        color: Option<HColor>,
    ) -> Rc<dyn Ftile> {
        let style = self.circle_style(SName::Spot);
        Rc::new(FtileCircleSpot::new(
            self.skin_param.clone(),
            swimlane,
            spot,
            color,
            style,
        ))
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
        Rc::new(FtileBox::create(
            self.skin_param.clone(),
            colors,
            label,
            swimlane,
            style,
            stereotype,
            style_builder,
        ))
    }

    /// The tile alone: notes are `FtileFactoryDelegatorAddNote`'s.
    fn add_note(
        &self,
        ftile: Option<Rc<dyn Ftile>>,
        swimlane: Option<SwimlaneId>,
        _notes: &[PositionedNote],
        _vertical_alignment: VerticalAlignment,
    ) -> Rc<dyn Ftile> {
        ftile.unwrap_or_else(|| Rc::new(FtileEmpty::new(self.skin_param.clone(), swimlane)))
    }

    fn add_url(&self, ftile: Rc<dyn Ftile>, _url: &Url) -> Rc<dyn Ftile> {
        ftile
    }

    fn decorate_in(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        Rc::new(FtileDecorateIn::new(ftile, link_rendering.clone()))
    }

    fn decorate_out(&self, ftile: Rc<dyn Ftile>, link_rendering: &LinkRendering) -> Rc<dyn Ftile> {
        Rc::new(FtileDecorateOut::new(ftile, link_rendering.clone()))
    }

    fn assembly(&self, tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<dyn Ftile> {
        Rc::new(FtileAssemblySimple::new(tile1, tile2))
    }

    fn repeat(
        &self,
        _stereotype: &Stereogroup,
        _stereotype2: &Stereogroup,
        _box_style_in: BoxStyle,
        _swimlane: Option<SwimlaneId>,
        _swimlane_out: Option<SwimlaneId>,
        _start_label: Option<&Display>,
        repeat: Rc<dyn Ftile>,
        _test: Option<&Display>,
        _yes: Option<&Display>,
        _out: Option<&Display>,
        _backward: Option<Rc<dyn Ftile>>,
        _no_out: bool,
        _incoming1: &LinkRendering,
        _incoming2: &LinkRendering,
        _current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        repeat
    }

    fn create_while(
        &self,
        _instructions: &Instructions,
        _out_color: &LinkRendering,
        _swimlane: Option<SwimlaneId>,
        while_block: Rc<dyn Ftile>,
        _test: &Display,
        _yes: Option<&Display>,
        _color: Option<HColor>,
        _special_out: Option<InstructionId>,
        _backward: Option<Rc<dyn Ftile>>,
        _incoming1: &LinkRendering,
        _incoming2: &LinkRendering,
        _current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        while_block
    }

    fn create_if(
        &self,
        _instructions: &Instructions,
        _swimlane: Option<SwimlaneId>,
        thens: &[BranchFtile<'_>],
        else_branch: &BranchFtile<'_>,
        _out_color: &LinkRendering,
        _top_inlink_rendering: &LinkRendering,
        _url: Option<&Url>,
        _notes: &[PositionedNote],
        _stereotype: Option<&Stereotype>,
        _current_style_builder: &Rc<StyleBuilder>,
    ) -> Rc<dyn Ftile> {
        let ftiles = thens
            .iter()
            .chain([else_branch])
            .map(|branch| branch.ftile.clone())
            .collect();
        Rc::new(FtileForkInner::new(ftiles))
    }

    fn create_switch(
        &self,
        _instructions: &Instructions,
        _swimlane: Option<SwimlaneId>,
        branches: &[BranchFtile<'_>],
        _after_endwhile: &LinkRendering,
        _top_inlink_rendering: &LinkRendering,
        _label_test: Option<&Display>,
        _colors: &Colors,
        _end_colors: Option<&Colors>,
    ) -> Rc<dyn Ftile> {
        let ftiles = branches.iter().map(|branch| branch.ftile.clone()).collect();
        Rc::new(FtileForkInner::new(ftiles))
    }

    fn create_parallel(
        &self,
        all: Vec<Rc<dyn Ftile>>,
        _style: ForkStyle,
        _label: Option<&str>,
        _swimlane_in: Option<SwimlaneId>,
        _swimlane_out: Option<SwimlaneId>,
        _colors: &Colors,
    ) -> Rc<dyn Ftile> {
        Rc::new(FtileForkInner::new(all))
    }

    fn create_group(
        &self,
        list: Rc<dyn Ftile>,
        _name: &Display,
        _back_color: Option<HColor>,
        _note: Option<&PositionedNote>,
        _type: USymbol,
        _style: &Style,
    ) -> Rc<dyn Ftile> {
        list
    }
}
