//! Conditionals: `if` with its `elseif` and `else` branches, and `switch` with its cases (PlantUML's
//! `Branch`, `InstructionIf` and `InstructionSwitch`).

use std::rc::Rc;

use super::instruction::{InstructionId, InstructionList, Instructions, WithNote};
use super::link_rendering::LinkRendering;
use super::swimlanes::SwimlaneId;
use crate::color::{ColorType, Colors, HColor};
use crate::creole::{CreoleMode, Display};
use crate::decoration::Rainbow;
use crate::ftile::Ftile;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;
use crate::stereo::Stereotype;
use crate::style::{SName, StyleBuilder, StyleSignature};

/// One way out of a conditional, and the instructions down it.
pub(crate) struct Branch {
    pub(crate) list: InstructionList,
    /// The condition shown in the diamond; an `else` has none.
    pub(crate) label_test: Option<Display>,
    /// The arrow into the branch, labelled with the condition's value.
    pub(crate) label_positive: LinkRendering,
    /// The arrow out of the branch's last instruction.
    pub(crate) inlink_rendering: LinkRendering,
    /// The arrow into an `elseif`'s diamond.
    pub(crate) inlabel: LinkRendering,
    /// The arrow the next branch or the end of the conditional set when it started.
    pub(crate) special: Option<LinkRendering>,
    pub(crate) color: Option<HColor>,
    pub(crate) special_colors: Option<Colors>,
}

impl Branch {
    pub(crate) fn new(
        swimlane: Option<SwimlaneId>,
        label_positive: LinkRendering,
        label_test: Option<Display>,
        color: Option<HColor>,
        inlabel: LinkRendering,
    ) -> Self {
        Self {
            list: InstructionList::new(swimlane),
            label_test,
            label_positive,
            inlink_rendering: LinkRendering::none(),
            inlabel,
            special: None,
            color,
            special_colors: None,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(crate) fn get_last(&self) -> Option<InstructionId> {
        self.list.get_last()
    }

    pub(crate) fn set_inlink_rendering(&mut self, inlink_rendering: LinkRendering) {
        self.inlink_rendering = inlink_rendering;
    }

    pub(crate) fn set_special(&mut self, link: LinkRendering, colors: Option<Colors>) {
        self.special = Some(link);
        self.special_colors = colors;
    }

    /// The colours of the arrow out of the branch (`getOut`).
    pub(crate) fn get_out(&self) -> Rainbow {
        self.special
            .as_ref()
            .unwrap_or(&self.inlink_rendering)
            .rainbow
            .clone()
    }

    /// The label of the arrow into an `elseif`'s diamond (`getInlabel`).
    pub(crate) fn get_inlabel(&self) -> Option<&Display> {
        self.inlabel.display.as_ref()
    }

    /// The colours of the arrow into an `elseif`'s diamond, or `default_color` (`getInRainbow`).
    pub(crate) fn get_in_rainbow(&self, default_color: &Rainbow) -> Rainbow {
        self.inlabel.get_rainbow_or(default_color)
    }

    /// The colour of the diamond: the one set at the end of the conditional, or else the branch's
    /// (`getColor`).
    pub(crate) fn get_color(&self) -> Option<HColor> {
        self.special_colors
            .as_ref()
            .and_then(|colors| colors.get(ColorType::Back))
            .or(self.color.as_ref())
            .cloned()
    }

    /// The label of the arrow out of the branch, if it has one (`getSpecialDisplay`).
    pub(crate) fn get_special_display(&self) -> Option<&Display> {
        self.special.as_ref()?.display.as_ref()
    }
}

/// A branch with the tile built for it, as factories get it (PlantUML's `Branch` once `updateFtile` set its
/// `ftile`). The tile is the branch's list, decorated with the branch's arrow out.
pub(crate) struct BranchFtile<'a> {
    pub(crate) branch: &'a Branch,
    pub(crate) ftile: Rc<dyn Ftile>,
}

impl BranchFtile<'_> {
    pub(crate) fn is_empty(&self) -> bool {
        self.branch.is_empty()
    }

    /// Whether the branch is a lone stop, end, spot or killed activity (`isOnlySingleStopOrSpot`).
    pub(crate) fn is_only_single_stop_or_spot(&self, instructions: &Instructions) -> bool {
        instructions.list_is_only_single_stop_or_spot(&self.branch.list)
    }

    /// The colours of the arrow into the branch, `arrow_color` by default (`getInColor`).
    pub(crate) fn get_in_color(&self, arrow_color: &Rainbow) -> Rainbow {
        if self.is_empty() {
            return self
                .ftile
                .get_out_link_rendering()
                .get_rainbow_or(arrow_color);
        }
        if self.branch.label_positive.rainbow.size() > 0 {
            return self.branch.label_positive.rainbow.clone();
        }
        let color = self
            .ftile
            .get_in_link_rendering()
            .get_rainbow_or(arrow_color);
        if color.size() == 0 {
            return arrow_color.clone();
        }
        color
    }

    /// The label of the arrow into the branch: the one on the branch's first arrow, or else the
    /// condition's value (`getDisplayPositive`).
    pub(crate) fn get_display_positive(&self) -> Option<Display> {
        self.ftile
            .get_in_link_rendering()
            .display
            .or_else(|| self.branch.label_positive.display.clone())
    }

    /// [`Self::get_display_positive`] in the arrow style (`getTextBlockPositive`).
    pub(crate) fn get_text_block_positive(&self) -> Rc<dyn TextBlock> {
        self.get_text_block(self.get_display_positive().as_ref())
    }

    /// The label of the arrow out of the branch in the arrow style (`getTextBlockSpecial`).
    pub(crate) fn get_text_block_special(&self) -> Rc<dyn TextBlock> {
        self.get_text_block(self.branch.get_special_display())
    }

    fn get_text_block(&self, display: Option<&Display>) -> Rc<dyn TextBlock> {
        let Some(display) = display else {
            return Rc::new(TextBlockEmpty::default());
        };
        let skin_param = self.ftile.skin_param();
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
        .get_merged_style(&skin_param.current_style_builder());
        Rc::new(display.create0(
            &style.font_configuration(),
            HorizontalAlignment::Left,
            skin_param.as_ref(),
            style.wrap_width(),
            CreoleMode::SimpleLine,
        ))
    }
}

pub(crate) struct InstructionIf {
    pub(crate) thens: Vec<Branch>,
    pub(crate) else_branch: Option<Branch>,
    /// Whether the else branch, rather than the last `then`, takes the next instructions.
    else_is_current: bool,
    pub(crate) endif_called: bool,
    pub(crate) url: Option<Url>,
    pub(crate) parent: InstructionId,
    pub(crate) top_inlink_rendering: LinkRendering,
    pub(crate) out_color: LinkRendering,
    pub(crate) stereotype: Option<Stereotype>,
    pub(crate) swimlane: Option<SwimlaneId>,
    pub(crate) current_style_builder: Rc<StyleBuilder>,
    pub(crate) notes: WithNote,
}

impl InstructionIf {
    #[expect(clippy::too_many_arguments, reason = "PlantUML's constructor")]
    pub(crate) fn new(
        swimlane: Option<SwimlaneId>,
        parent: InstructionId,
        label_test: Option<Display>,
        when_then: LinkRendering,
        inlink_rendering: LinkRendering,
        color: Option<HColor>,
        current_style_builder: Rc<StyleBuilder>,
        url: Option<Url>,
        stereotype: Option<Stereotype>,
    ) -> Self {
        Self {
            thens: vec![Branch::new(
                swimlane,
                when_then,
                label_test,
                color,
                LinkRendering::none(),
            )],
            else_branch: None,
            else_is_current: false,
            endif_called: false,
            url,
            parent,
            top_inlink_rendering: inlink_rendering,
            out_color: LinkRendering::none(),
            stereotype,
            swimlane,
            current_style_builder,
            notes: WithNote::default(),
        }
    }

    /// The branch taking the next instructions.
    pub(crate) fn current(&self) -> &Branch {
        match &self.else_branch {
            Some(else_branch) if self.else_is_current => else_branch,
            _ => self.thens.last().expect("an if has a first branch"),
        }
    }

    pub(crate) fn current_mut(&mut self) -> &mut Branch {
        match &mut self.else_branch {
            Some(else_branch) if self.else_is_current => else_branch,
            _ => self.thens.last_mut().expect("an if has a first branch"),
        }
    }

    /// Starts the else branch, unless there is one already.
    pub(crate) fn switch_to_else2(
        &mut self,
        when_else: LinkRendering,
        next_link_renderer: LinkRendering,
    ) -> bool {
        self.current_mut()
            .set_special(next_link_renderer.clone(), None);
        if self.else_branch.is_some() {
            return false;
        }
        self.current_mut().set_inlink_rendering(next_link_renderer);
        self.else_branch = Some(Branch::new(
            self.swimlane,
            when_else,
            None,
            None,
            LinkRendering::none(),
        ));
        self.else_is_current = true;
        true
    }

    /// Starts an `elseif` branch, unless the else branch has started.
    pub(crate) fn else_if(
        &mut self,
        inlabel: LinkRendering,
        test: Option<Display>,
        when_then: LinkRendering,
        next_link_renderer: LinkRendering,
        color: Option<HColor>,
    ) -> bool {
        if self.else_branch.is_some() {
            return false;
        }
        self.current_mut().set_special(next_link_renderer, None);
        self.thens
            .push(Branch::new(self.swimlane, when_then, test, color, inlabel));
        true
    }

    pub(crate) fn endif(&mut self, next_link_renderer: LinkRendering, colors: Colors) {
        self.endif_called = true;
        let swimlane = self.swimlane;
        self.else_branch
            .get_or_insert_with(|| {
                Branch::new(
                    swimlane,
                    LinkRendering::none(),
                    None,
                    None,
                    LinkRendering::none(),
                )
            })
            .set_special(next_link_renderer.clone(), Some(colors));
        self.current_mut().set_inlink_rendering(next_link_renderer);
    }

    pub(crate) fn get_last(&self) -> Option<InstructionId> {
        match &self.else_branch {
            Some(else_branch) => else_branch.get_last(),
            None => self.thens.last().and_then(Branch::get_last),
        }
    }

    pub(crate) fn out_color(&mut self, out_color: LinkRendering) {
        self.out_color = out_color;
    }
}

pub(crate) struct InstructionSwitch {
    /// The cases, the last taking the next instructions.
    pub(crate) switches: Vec<Branch>,
    pub(crate) parent: InstructionId,
    pub(crate) top_inlink_rendering: LinkRendering,
    pub(crate) label_test: Option<Display>,
    pub(crate) swimlane: Option<SwimlaneId>,
    pub(crate) colors: Colors,
    pub(crate) end_colors: Option<Colors>,
    pub(crate) notes: WithNote,
}

impl InstructionSwitch {
    pub(crate) fn new(
        swimlane: Option<SwimlaneId>,
        parent: InstructionId,
        label_test: Option<Display>,
        inlink_rendering: LinkRendering,
        colors: Colors,
    ) -> Self {
        Self {
            switches: Vec::new(),
            parent,
            top_inlink_rendering: inlink_rendering,
            label_test,
            swimlane,
            colors,
            end_colors: None,
            notes: WithNote::default(),
        }
    }

    pub(crate) fn switch_case(
        &mut self,
        label_case: Option<Display>,
        next_link_renderer: LinkRendering,
    ) {
        if let Some(current) = self.switches.last_mut() {
            current.set_special(next_link_renderer, None);
        }
        let label = LinkRendering::none().with_display(label_case.clone());
        self.switches.push(Branch::new(
            self.swimlane,
            label.clone(),
            label_case,
            None,
            label,
        ));
    }

    pub(crate) fn end_switch(&mut self, next_link_renderer: LinkRendering, end_colors: Colors) {
        if let Some(current) = self.switches.last_mut() {
            current.set_special(next_link_renderer, Some(end_colors.clone()));
        }
        self.end_colors = Some(end_colors);
    }
}
