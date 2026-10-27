//! Wire diagrams, `@startwire`: nested blocks stacked in columns, texts printed between them, spots on them,
//! and wires and buses from one block to the side of another (PlantUML's undocumented `wire` package and
//! `WireDiagramFactory`).

mod block;
mod commands;
mod link;

use std::rc::Rc;
use std::sync::{Arc, LazyLock};

use self::block::{Located, Spot, WBlock};
use self::link::{WArrowDirection, WLinkHorizontal, WLinkType, WLinkVertical};
use super::builder::CommandFactory;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult};
use crate::creole::Display;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::typeface::{FontRegistry, StringBounderFonts};
use crate::klimt::ugraphic::UGraphic;
use crate::style::SName;

/// Reads wire diagrams (PlantUML's `WireDiagramFactory`).
pub(super) struct WireDiagramFactory;

impl CommandFactory for WireDiagramFactory {
    type Diagram = WireDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Wire;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> WireDiagram {
        WireDiagram {
            source: source.clone(),
            titled: Titled::new(SName::ActivityDiagram, "WIRE", source),
            root: WBlock::root(),
            spots: Vec::new(),
            hlinks: Vec::new(),
            vlinks: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<WireDiagram>>> {
        commands::init_commands_list()
    }
}

/// A `@startwire` diagram (PlantUML's `WireDiagram`).
pub(super) struct WireDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    root: WBlock,
    spots: Vec<Spot>,
    hlinks: Vec<WLinkHorizontal>,
    vlinks: Vec<WLinkVertical>,
}

/// Four spaces or a tab make a level, and so does any other space.
fn compute_indentation_level(indent: &str) -> usize {
    indent.replace("    ", "\t").chars().count()
}

/// PlantUML measures printed texts as it would for PNG whatever the output (`FileFormat.PNG
/// .getDefaultStringBounder()`): with the embedded fonts here.
fn png_string_bounder() -> StringBounderFonts {
    static FONTS: LazyLock<Arc<FontRegistry>> = LazyLock::new(Arc::default);
    StringBounderFonts::new(FONTS.clone())
}

fn no_such_element(name: &str) -> CommandError {
    CommandError::new(format!("No such element {name}"))
}

/// What a link command read.
struct LinkSpec<'a> {
    from: &'a str,
    at: Option<(&'a str, &'a str)>,
    to: &'a str,
    link_type: WLinkType,
    direction: WArrowDirection,
    color: HColor,
}

/// The two blocks a link joins.
fn get_blocks<'a>(
    root: &'a WBlock,
    spec: &LinkSpec,
) -> Result<(Located<'a>, Located<'a>), CommandError> {
    let block1 = root
        .get_block(spec.from)
        .ok_or_else(|| no_such_element(spec.from))?;
    let block2 = root
        .get_block(spec.to)
        .ok_or_else(|| no_such_element(spec.to))?;
    Ok((block1, block2))
}

impl WireDiagram {
    fn add_component(
        &mut self,
        indent: &str,
        name: &str,
        width: f64,
        height: f64,
        color: Option<HColor>,
    ) -> CommandResult {
        self.root.add_block(
            compute_indentation_level(indent),
            name,
            width,
            height,
            color,
        )
    }

    fn new_column(&mut self, indent: &str) -> CommandResult {
        self.root.new_column(compute_indentation_level(indent))
    }

    fn spot(
        &mut self,
        name: &str,
        color: Option<HColor>,
        at: Option<(&str, &str)>,
    ) -> CommandResult {
        if self.root.get_block(name).is_none() {
            return Err(no_such_element(name));
        }
        let (x, y) = at.unwrap_or(("0", "0"));
        self.spots.push(Spot {
            block: name.to_owned(),
            color: color.unwrap_or(HColor::RED),
            x: x.to_owned(),
            y: y.to_owned(),
        });
        Ok(())
    }

    fn wgoto(&mut self, indent: &str, x: f64, y: f64) -> CommandResult {
        self.root.wgoto(compute_indentation_level(indent), x, y)
    }

    fn wmove(&mut self, indent: &str, x: f64, y: f64) -> CommandResult {
        self.root.wmove(compute_indentation_level(indent), x, y)
    }

    fn print(&mut self, indent: &str, text: &str) -> CommandResult {
        self.root.print(
            &png_string_bounder(),
            &self.titled.skin,
            compute_indentation_level(indent),
            Display::with_newlines(text),
        )
    }

    fn vlink(&mut self, spec: LinkSpec) -> CommandResult {
        let (block1, block2) = get_blocks(&self.root, &spec)?;
        self.vlinks.push(WLinkVertical {
            start: block1.get_next_out_vertical(spec.at, spec.link_type),
            destination: block2.get_absolute_position("0", "0").dy,
            link_type: spec.link_type,
            direction: spec.direction,
            color: spec.color,
        });
        Ok(())
    }

    fn hlink(&mut self, spec: LinkSpec, label: Display) -> CommandResult {
        let (block1, block2) = get_blocks(&self.root, &spec)?;
        self.hlinks.push(WLinkHorizontal {
            start: block1.get_next_out_horizontal(spec.at, spec.link_type),
            destination: block2.get_absolute_position("0", "0").dx,
            link_type: spec.link_type,
            direction: spec.direction,
            color: spec.color,
            label,
        });
        Ok(())
    }

    fn draw_me(&self, ug: &UGraphic) {
        let skin = &self.titled.skin;
        self.root.draw_me(ug, skin);
        for spot in &self.spots {
            spot.draw_me(ug, &self.root);
        }
        for link in &self.hlinks {
            link.draw_me(ug, skin);
        }
        for link in &self.vlinks {
            link.draw_me(ug);
        }
    }
}

/// The whole drawing, as large as the root block.
struct Drawing<'a> {
    diagram: &'a WireDiagram,
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> XDimension2D {
        self.diagram.root.get_max_dimension()
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.diagram.draw_me(ug);
    }
}

impl AbstractDiagram for WireDiagram {}

impl TitledDiagram for WireDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for WireDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        Ok(self
            .titled
            .add_chrome(Box::new(Drawing { diagram: self }), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_spaces_or_a_tab_make_a_level_and_so_does_any_other_space() {
        assert_eq!(compute_indentation_level(""), 0);
        assert_eq!(compute_indentation_level("    "), 1);
        assert_eq!(compute_indentation_level("\t"), 1);
        assert_eq!(compute_indentation_level("  "), 2);
        assert_eq!(compute_indentation_level("     "), 2);
    }
}
