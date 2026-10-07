//! Class and object diagrams (PlantUML's `classdiagram` and `objectdiagram` packages). Maps and JSON
//! objects are read here too.

mod commands;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::{
    add_common_commands2, add_common_hides, add_common_scale_commands, add_title_commands,
};
use super::cuca::{AbstractClassOrObjectDiagram, CucaDiagram, EntityDiagram};
use super::cuca_commands::{self, note};
use super::diagram_type::DiagramType;
use super::titled::{PragmaKey, Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::pattern::RegexTree;
use crate::style::SName;

/// Drawing class diagrams is not ported yet; it is reported as soon as the lines read as one.
const NOT_PORTED: NotYetPorted = NotYetPorted("class diagrams");

pub(super) struct ClassDiagram {
    source: Rc<UmlSource>,
    diagram: AbstractClassOrObjectDiagram,
    /// Description elements may appear among the classes.
    allow_mixing: bool,
}

/// Reads class and object diagrams (PlantUML's `ClassDiagramFactory`).
pub(super) struct ClassDiagramFactory;

impl CommandFactory for ClassDiagramFactory {
    type Diagram = ClassDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Class;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ClassDiagram {
        let mut titled = Titled::new(SName::ClassDiagram, "CLASS", source);
        titled.not_ported(NOT_PORTED);
        ClassDiagram {
            source: source.clone(),
            diagram: AbstractClassOrObjectDiagram::new(titled),
            allow_mixing: false,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<ClassDiagram>>> {
        let mut commands = vec![
            cuca_commands::footbox_ignored(),
            cuca_commands::rank_dir(),
            cuca_commands::newpage(),
        ];
        commands.extend(add_common_scale_commands());
        commands.push(commands::add_method());
        commands.extend(add_common_hides());
        commands.extend([
            cuca_commands::hide_show2(),
            cuca_commands::remove_restore(),
            commands::create_class_multilines(),
            commands::create_entity_object_multilines(),
            cuca_commands::create_map(),
            cuca_commands::create_json(),
            cuca_commands::create_json_single_line(),
            commands::create_class(),
            commands::create_entity_object(),
            commands::allow_mixing(),
            commands::create_element_parenthesis(),
            commands::layout_new_line(),
            commands::package(),
            cuca_commands::end_package(),
            commands::package_empty(),
            cuca_commands::package_with_usymbol(),
            cuca_commands::together(),
            commands::create_element_full2(commands::Mode::NormalKeyword),
            commands::create_element_full2(commands::Mode::WithMixPrefix),
            note::note(),
            commands::namespace(),
            commands::namespace2(),
            commands::namespace_empty(),
            commands::stereotype(),
            commands::link_class(),
            commands::link_lollipop(),
            note::tip_on_entity_multi_line(true),
            note::tip_on_entity_multi_line(false),
            note::note_on_entity(code_for_class, ParserPass::One),
            cuca_commands::url(),
            note::note_on_entity_multi_line(code_for_class, ParserPass::One, true),
            note::note_on_entity_multi_line(code_for_class, ParserPass::One, false),
            note::note_multi_line(),
            note::note_on_link(ParserPass::One),
            note::note_on_link_multi_line(ParserPass::One),
            note::constraint_on_links(),
            commands::diamond_association(),
            cuca_commands::create_element_multilines_type0(),
            cuca_commands::create_element_multilines_type1(),
        ]);
        commands.extend(add_title_commands());
        commands.extend(add_common_commands2());
        commands
    }
}

/// How notes name the class they are on (`NameAndCodeParser.codeForClass`).
fn code_for_class() -> RegexTree {
    RegexTree::named(1, "CODE", "([^%s{}%g<>]+|[%g][^%g]+[%g])")
}

impl AbstractDiagram for ClassDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {
        self.diagram.cuca.starting_pass();
    }

    /// Links between the same entities all get the length 1 if one of them has it.
    fn check_final_error(&mut self) -> Option<String> {
        let cuca = &mut self.diagram.cuca;
        let links = cuca.get_link_ids().to_vec();
        for &link in &links {
            if cuca.link(link).get_length() != 1 {
                continue;
            }
            for &link2 in &links {
                if cuca.link(link2).same_connections(cuca.link(link))
                    && cuca.link(link2).get_length() != 1
                {
                    cuca.link_mut(link2).set_length(1);
                }
            }
        }
        if cuca
            .titled
            .pragma
            .is_false(PragmaKey::UseIntermediatePackages)
        {
            cuca.pack_some_package();
        }
        cuca.apply_single_strategy();
        None
    }
}

impl EntityDiagram for ClassDiagram {
    fn cuca(&mut self) -> &mut CucaDiagram {
        &mut self.diagram.cuca
    }
}

impl TitledDiagram for ClassDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.diagram.cuca.titled
    }
}

impl Diagram for ClassDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        _string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Err(NOT_PORTED)
    }

    fn export_settings(&self) -> ExportSettings {
        self.diagram
            .cuca
            .titled
            .export_settings(self.source.seed(), CucaDiagram::get_default_margins())
    }
}
