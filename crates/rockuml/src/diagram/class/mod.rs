//! Class and object diagrams (PlantUML's `classdiagram` and `objectdiagram` packages). Maps and JSON
//! objects are read here too.

mod commands;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::{
    add_common_commands2, add_common_hides, add_common_scale_commands, add_title_commands,
};
use super::cuca_commands::{self, note};
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::pattern::RegexTree;
use crate::style::SName;

pub(super) struct ClassDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
}

/// Reads class and object diagrams (PlantUML's `ClassDiagramFactory`).
pub(super) struct ClassDiagramFactory;

impl CommandFactory for ClassDiagramFactory {
    type Diagram = ClassDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Class;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ClassDiagram {
        ClassDiagram {
            titled: Titled::new(SName::ClassDiagram, "CLASS", source),
            source: source.clone(),
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
            commands::create_element_full2_normal_keyword(),
            commands::create_element_full2_with_mix_prefix(),
            note::note(),
            commands::namespace(),
            commands::namespace2(),
            commands::namespace_empty(),
            commands::stereotype(),
            commands::link_class(),
            commands::link_lollipop(),
            commands::tip_on_entity_multi_line_with_bracket(),
            commands::tip_on_entity_multi_line(),
            note::note_on_entity(code_for_class(), ParserPass::One),
            cuca_commands::url(),
            note::note_on_entity_multi_line(code_for_class(), ParserPass::One, true),
            note::note_on_entity_multi_line(code_for_class(), ParserPass::One, false),
            note::note_multi_line(),
            note::note_on_link(ParserPass::One),
            note::note_on_link_multi_line(ParserPass::One),
            commands::constraint_on_links(),
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

impl AbstractDiagram for ClassDiagram {}

impl TitledDiagram for ClassDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
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
        Err(NotYetPorted("class diagrams"))
    }

    fn export_settings(&self) -> ExportSettings {
        self.titled.export_settings(
            self.source.seed(),
            ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 5.0, 5.0, 0.0),
        )
    }
}
