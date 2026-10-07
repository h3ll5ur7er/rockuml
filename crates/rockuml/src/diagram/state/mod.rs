//! State diagrams (PlantUML's `statediagram` package). Their lines are read three times: links may name
//! states declared further down, and notes may name states links created.

mod commands;
#[cfg(test)]
mod graph_tests;
#[cfg(test)]
mod image_tests;
#[cfg(test)]
mod tests;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::cuca::{CucaDiagram, EntityDiagram};
use super::cuca_commands::{self, note};
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::abel::{EntityId, GroupType, LeafType};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, ParserPass};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::pattern::RegexTree;
use crate::plasma::QuarkId;
use crate::style::SName;
use crate::text::LineLocation;

/// Names the groups of concurrent regions, numbered afresh on each pass.
const CONCURRENT_PREFIX: &str = "CONC";

pub(super) struct StateDiagram {
    source: Rc<UmlSource>,
    cuca: CucaDiagram,
    /// Commands that run in several passes do some of their work in the first only.
    current_pass: ParserPass,
}

/// Reads state diagrams (PlantUML's `StateDiagramFactory`).
pub(super) struct StateDiagramFactory;

impl CommandFactory for StateDiagramFactory {
    type Diagram = StateDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::State;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> StateDiagram {
        let titled = Titled::new(SName::StateDiagram, "STATE", source);
        let mut cuca = CucaDiagram::new(titled);
        cuca.set_namespace_separator(Some("."));
        StateDiagram {
            source: source.clone(),
            cuca,
            current_pass: ParserPass::One,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<StateDiagram>>> {
        let mut commands = vec![
            cuca_commands::footbox_ignored(),
            cuca_commands::rank_dir(),
            cuca_commands::remove_restore(),
            commands::create_state(),
            commands::link_state(),
            commands::link_state_reverse(),
            commands::create_package_state(),
            commands::create_package2(),
            commands::end_state(),
            commands::add_field(),
            commands::concurrent_state(),
            note::note_on_entity_multi_line(code_for_state, ParserPass::Three, true),
            note::note_on_entity_multi_line(code_for_state, ParserPass::Three, false),
            note::note_on_entity(code_for_state, ParserPass::Three),
            note::note_on_link(ParserPass::Two),
            note::note_on_link_multi_line(ParserPass::Two),
            cuca_commands::url(),
            note::note(),
            note::note_multi_line(),
            cuca_commands::create_map(),
            cuca_commands::create_json(),
            cuca_commands::create_json_single_line(),
        ];
        commands.extend(add_common_commands1());
        commands.push(cuca_commands::hide_show2());
        commands
    }
}

/// How notes name the state they are on.
fn code_for_state() -> RegexTree {
    RegexTree::named_or(
        "CODE",
        vec![
            RegexTree::leaf("[%pLN_.]+"),
            RegexTree::leaf("[%g][^%g]+[%g]"),
        ],
    )
}

impl StateDiagram {
    /// Whether a state named by `quark` may be used in the current group: a state in a concurrent region
    /// stays in it, and a concurrent region uses only its own states.
    fn check_concurrent_state_ok(&self, quark: QuarkId) -> Result<bool, CommandError> {
        let cuca = &self.cuca;
        let Some(existing) = cuca.quark(quark).get_data() else {
            return Ok(true);
        };
        let current = cuca.get_current_group();
        let parent = cuca.entity(existing).get_parent_container(cuca);
        if is_concurrent_state(cuca, current)? && Some(current) != parent {
            return Ok(false);
        }
        Ok(match parent {
            Some(parent) => !is_concurrent_state(cuca, parent)? || current == parent,
            None => true,
        })
    }

    /// The pseudo-state `prefix` names in the current group, like `*start*` at the root or
    /// `*start*Active` in the state `Active`, created as a leaf of `leaf_type` if needed.
    fn pseudo_state(
        &mut self,
        location: &LineLocation,
        prefix: &str,
        leaf_type: LeafType,
    ) -> Result<EntityId, CommandError> {
        let group = self.cuca.get_current_group();
        let id_short = if self.cuca.entity(group).is_root() {
            prefix.to_owned()
        } else {
            format!("{prefix}{}", self.cuca.entity(group).get_name(&self.cuca))
        };
        let quark = self
            .cuca
            .quark_in_context(true, Self::clean_id(&id_short))?;
        Ok(self.leaf_of(location, quark, leaf_type))
    }

    /// The entity `quark` holds, or a new leaf of `leaf_type` without a name shown.
    fn leaf_of(
        &mut self,
        location: &LineLocation,
        quark: QuarkId,
        leaf_type: LeafType,
    ) -> EntityId {
        self.leaf_named(location, quark, Display::with_newlines(""), leaf_type)
    }

    /// The entity `quark` holds, or a new leaf of `leaf_type` showing `display`.
    fn leaf_named(
        &mut self,
        location: &LineLocation,
        quark: QuarkId,
        display: Display,
        leaf_type: LeafType,
    ) -> EntityId {
        match self.cuca.quark(quark).get_data() {
            Some(existing) => existing,
            None => self
                .cuca
                .really_create_leaf(Some(location), quark, display, leaf_type),
        }
    }

    /// `[*]` where a transition starts.
    fn get_start(&mut self, location: &LineLocation) -> Result<EntityId, CommandError> {
        self.pseudo_state(location, "*start*", LeafType::CircleStart)
    }

    /// `[*]` where a transition ends.
    fn get_end(&mut self, location: &LineLocation) -> Result<EntityId, CommandError> {
        self.pseudo_state(location, "*end*", LeafType::CircleEnd)
    }

    /// `[H]`: the shallow history of the current group.
    fn get_historical(&mut self, location: &LineLocation) -> Result<EntityId, CommandError> {
        self.pseudo_state(location, "*historical*", LeafType::PseudoState)
    }

    /// `[H*]`: the deep history of the current group.
    fn get_deep_history(&mut self, location: &LineLocation) -> Result<EntityId, CommandError> {
        self.pseudo_state(location, "*deephistory*", LeafType::DeepHistory)
    }

    /// `State[H]` and `State[H*]`: the history of `id_short`, which becomes a composite state.
    fn get_history_of(
        &mut self,
        location: &LineLocation,
        id_short: &str,
        prefix: &str,
        leaf_type: LeafType,
    ) -> Result<EntityId, CommandError> {
        let quark = self.cuca.quark_in_context(true, Self::clean_id(id_short))?;
        let display = Display::with_newlines(self.cuca.quark(quark).get_name());
        self.cuca
            .goto_group(Some(location), quark, display, GroupType::State);
        let group = self.cuca.get_current_group();
        let name = format!("{prefix}{}", self.cuca.entity(group).get_name(&self.cuca));
        let ident = self.cuca.quark_in_context(true, Self::clean_id(&name))?;
        let result = self.leaf_of(location, ident, leaf_type);
        self.end_group()?;
        Ok(result)
    }

    /// `--` or `||`: the current state's next concurrent region starts, separated by a horizontal or a
    /// vertical line.
    fn concurrent_state(&mut self, location: &LineLocation, direction: char) -> CommandResult {
        let current = self.cuca.get_current_group();
        self.cuca.entity_mut(current).concurrent_separator = Some(direction);
        if is_concurrent_state(&self.cuca, current)? {
            self.cuca.end_group();
        }
        let name = self.cuca.get_unique_sequence2(CONCURRENT_PREFIX);
        let ident = self.cuca.quark_in_context(true, Self::clean_id(&name))?;
        self.cuca.goto_group(
            Some(location),
            ident,
            Display::create([""]),
            GroupType::ConcurrentState,
        );
        let region = self.cuca.get_current_group();
        self.cuca.entity_mut(region).concurrent_separator = Some(direction);
        Ok(())
    }

    /// Leaves the current state, and the concurrent region the commands were in; whether there was a state.
    fn end_group(&mut self) -> Result<bool, CommandError> {
        let current = self.cuca.get_current_group();
        if is_concurrent_state(&self.cuca, current)? {
            self.cuca.end_group();
        }
        Ok(self.cuca.end_group())
    }

    /// Every quark above `current` that holds nothing yet becomes a composite state named after it.
    fn ensure_parent_state(&mut self, location: &LineLocation, mut current: QuarkId) {
        while let Some(parent) = self.cuca.quark(current).get_parent() {
            if self.cuca.quark(parent).get_data().is_some() {
                return;
            }
            let display = Display::with_newlines(self.cuca.quark(parent).get_name());
            let group = self
                .cuca
                .create_group(Some(location), parent, GroupType::State);
            self.cuca.entity_mut(group).display = display;
            current = parent;
        }
    }
}

impl AbstractDiagram for StateDiagram {
    fn required_pass(&self) -> &'static [ParserPass] {
        &[ParserPass::One, ParserPass::Two, ParserPass::Three]
    }

    fn starting_pass(&mut self, pass: ParserPass) {
        self.current_pass = pass;
        self.cuca.starting_pass();
    }

    /// A link may not leave the concurrent region its state is in.
    fn check_final_error(&mut self) -> Option<String> {
        let cuca = &self.cuca;
        for link in cuca.get_links() {
            let region1 = concurrent_region(cuca, link.get_entity1());
            let region2 = concurrent_region(cuca, link.get_entity2());
            match (region1, region2) {
                (Err(error), _) | (_, Err(error)) => return Some(error.message),
                (Ok(region1), Ok(region2)) if region1 != region2 => {
                    return Some(format!(
                        "State within concurrent state cannot be linked out of this concurrent state (between {} and {})",
                        cuca.entity(link.get_entity1()).get_name(cuca),
                        cuca.entity(link.get_entity2()).get_name(cuca)
                    ));
                }
                _ => {}
            }
        }
        None
    }
}

/// The innermost concurrent region around `entity` (`getGroupParentIfItIsConcurrentState`).
fn concurrent_region(
    cuca: &CucaDiagram,
    entity: EntityId,
) -> Result<Option<EntityId>, CommandError> {
    let mut parent = cuca.entity(entity).get_parent_container(cuca);
    while let Some(group) = parent {
        if is_concurrent_state(cuca, group)? {
            return Ok(Some(group));
        }
        parent = cuca.entity(group).get_parent_container(cuca);
    }
    Ok(None)
}

/// Whether `entity` is a concurrent region. A state below a simple state, which only a dotted name can put
/// there, has no type of group to tell, and PlantUML fails on it (`Entity.checkGroup`).
fn is_concurrent_state(cuca: &CucaDiagram, entity: EntityId) -> Result<bool, CommandError> {
    let entity = cuca.entity(entity);
    if !entity.is_group() {
        return Err(CommandError::new(format!(
            "{} is not a composite state",
            entity.get_name(cuca)
        )));
    }
    Ok(entity.get_group_type() == GroupType::ConcurrentState)
}

impl EntityDiagram for StateDiagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl TitledDiagram for StateDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.cuca.titled
    }

    fn set_hide_empty_description(&mut self, hide: bool) {
        self.cuca.set_hide_empty_description_for_state(hide);
    }
}

impl Diagram for StateDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let drawing = self.cuca.get_text_block(string_bounder.as_ref())?;
        Ok(self.cuca.titled.add_chrome(drawing, string_bounder))
    }

    fn export_settings(&self) -> ExportSettings {
        self.cuca
            .titled
            .export_settings(self.source.seed(), CucaDiagram::get_default_margins())
    }
}
