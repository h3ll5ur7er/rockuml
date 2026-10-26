//! Flow diagrams, `@startflow`: boxes laid out on a grid, each a step north, south, east or west of the one
//! before, and links between them (PlantUML's undocumented `flowdiagram` package and `FlowDiagramFactory`).

mod golem;

use std::rc::Rc;

use golem::{Path, Tile, TileArea, TileGeometry, TilesField};

use super::builder::CommandFactory;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, CommandError, CommandResult, PatternCommand, SingleLine};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XPoint2D};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::pattern::{RegexResult, RegexTree};
use crate::style::SName;
use crate::text::LineLocation;

/// The room of one grid cell.
const SINGLE_SIZE_X: f64 = 100.0;
const SINGLE_SIZE_Y: f64 = 35.0;

/// Reads flow diagrams (PlantUML's `FlowDiagramFactory`).
pub(super) struct FlowDiagramFactory;

impl CommandFactory for FlowDiagramFactory {
    type Diagram = FlowDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Flow;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> FlowDiagram {
        FlowDiagram {
            source: source.clone(),
            titled: Titled::new(SName::ActivityDiagram, "FLOW", source),
            field: None,
            tiles_boxes: Vec::new(),
            last_tile: TilesField::ROOT,
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<FlowDiagram>>> {
        vec![command_line_simple(), command_link()]
    }
}

/// The optional side a command starts with, `s` when there is none.
fn orientation(arg: &RegexResult) -> TileGeometry {
    arg.get("ORIENTATION", 0)
        .map_or(TileGeometry::South, TileGeometry::from_letter)
}

fn optional_orientation() -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::named(1, "ORIENTATION", "([nsew])"),
        RegexTree::spaces_one_or_more(),
    ]))
}

/// PlantUML's `CommandLineSimple`: a new box, with its id and quoted label, on a side of the last one.
fn command_line_simple() -> Box<dyn Command<FlowDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            optional_orientation(),
            RegexTree::named(1, "ID_DEST", r"(\w+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "LABEL", "[%g](.*)[%g]"),
            RegexTree::end(),
        ]),
        |diagram: &mut FlowDiagram, _: &LineLocation, arg: &RegexResult| {
            diagram.line_simple(
                orientation(arg),
                arg.get("ID_DEST", 0).unwrap_or_default(),
                arg.get("LABEL", 0).unwrap_or_default(),
            )
        },
    )))
}

/// PlantUML's `CommandLink`: a link from a side of the last box to the box with that id.
fn command_link() -> Box<dyn Command<FlowDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            optional_orientation(),
            RegexTree::named(1, "ID_DEST", r"(\w+)"),
            RegexTree::end(),
        ]),
        |diagram: &mut FlowDiagram, _: &LineLocation, arg: &RegexResult| {
            diagram.link_simple(orientation(arg), arg.get("ID_DEST", 0).unwrap_or_default())
        },
    )))
}

/// What PlantUML's `ProtectedCommand` reports of the `IllegalArgumentException` the grid throws.
fn illegal_argument(message: Option<&str>) -> CommandError {
    let exception = "java.lang.IllegalArgumentException";
    CommandError::thrown(&match message {
        Some(message) => format!("{exception}: {message}"),
        None => exception.to_owned(),
    })
}

/// A `@startflow` diagram (PlantUML's `FlowDiagram`).
///
/// PlantUML keeps the boxes in a `HashMap` keyed by tiles, which hash by identity: the order it draws them
/// in, the box an id finds when two share it, and where boxes moved east to make room end up change from one
/// Java machine to another. Here the boxes come in the order they were declared.
pub(super) struct FlowDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    field: Option<TilesField>,
    /// The box of each tile, by tile number.
    tiles_boxes: Vec<ActivityBox>,
    last_tile: Tile,
}

impl FlowDiagram {
    fn line_simple(
        &mut self,
        orientation: TileGeometry,
        id_dest: &str,
        label: &str,
    ) -> CommandResult {
        let new_tile = match &mut self.field {
            None => {
                self.field = Some(TilesField::new());
                TilesField::ROOT
            }
            Some(field) => field
                .create_tile(self.last_tile, orientation)
                .map_err(|_| illegal_argument(None))?,
        };
        self.tiles_boxes.push(ActivityBox::new(id_dest, label));
        self.last_tile = new_tile;
        Ok(())
    }

    fn link_simple(&mut self, orientation: TileGeometry, id_dest: &str) -> CommandResult {
        let tile = self.get_tile_by_id(id_dest)?;
        self.field
            .as_mut()
            .expect("a box was found, so the grid exists")
            .add_path(self.last_tile, tile, orientation)
            .map_err(|_| illegal_argument(None))
    }

    fn get_tile_by_id(&self, id: &str) -> Result<Tile, CommandError> {
        self.tiles_boxes
            .iter()
            .position(|activity_box| activity_box.id == id)
            .map(Tile)
            .ok_or_else(|| illegal_argument(Some(id)))
    }
}

/// The drawing of a flow diagram that has boxes.
struct Drawing<'a> {
    field: &'a TilesField,
    tiles_boxes: &'a [ActivityBox],
}

/// The grid cells the tiles cover (`MinMaxGolem`).
struct MinMaxGolem {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

impl MinMaxGolem {
    fn width(&self) -> i32 {
        self.max_x - self.min_x + 1
    }

    fn height(&self) -> i32 {
        self.max_y - self.min_y + 1
    }
}

impl Drawing<'_> {
    fn tiles(&self) -> impl Iterator<Item = (Tile, &ActivityBox)> {
        self.tiles_boxes
            .iter()
            .enumerate()
            .map(|(tile, activity_box)| (Tile(tile), activity_box))
    }

    fn get_min_max(&self) -> MinMaxGolem {
        let mut min_max = MinMaxGolem {
            min_x: i32::MAX,
            min_y: i32::MAX,
            max_x: i32::MIN,
            max_y: i32::MIN,
        };
        for (tile, _) in self.tiles() {
            let position = self.field.get_position(tile);
            min_max.min_x = min_max.min_x.min(position.xmin);
            min_max.min_y = min_max.min_y.min(position.ymin);
            min_max.max_x = min_max.max_x.max(position.xmax);
            min_max.max_y = min_max.max_y.max(position.ymax);
        }
        min_max
    }

    fn get_center(&self, area: TileArea) -> XPoint2D {
        let position = self.field.get_position(area.tile);
        XPoint2D::new(
            f64::from(position.center_x()) * SINGLE_SIZE_X,
            f64::from(position.center_y()) * SINGLE_SIZE_Y,
        )
    }

    /// The middle of the tile's box side.
    fn move_point(&self, area: TileArea, string_bounder: &dyn StringBounder) -> XPoint2D {
        let XPoint2D { mut x, mut y } = self.get_center(area);
        let dim = self.tiles_boxes[area.tile.0].calculate_dimension(string_bounder);
        match area.geometry {
            TileGeometry::South => y += dim.height / 2.0,
            TileGeometry::North => y -= dim.height / 2.0,
            TileGeometry::East => x += dim.width / 2.0,
            TileGeometry::West => x -= dim.width / 2.0,
        }
        XPoint2D::new(x, y)
    }

    /// A line from side to side, a dot where it arrives; `(x, y)` is where the grid's top left cell lies.
    fn draw_path(&self, ug: &UGraphic, (x, y): (f64, f64), path: &Path) {
        let string_bounder = ug.string_bounder();
        let p_start = self.move_point(path.start, string_bounder);
        let p_dest = self.move_point(path.dest, string_bounder);
        let line = UShape::Line {
            dx: p_dest.x - p_start.x,
            dy: p_dest.y - p_start.y,
        };
        ug.translated(x + p_start.x, y + p_start.y).draw(&line);
        ug.translated(x + p_dest.x - 3.0, y + p_dest.y - 3.0)
            .draw(&UShape::Ellipse(UEllipse::new(7.0, 7.0)));
    }
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> XDimension2D {
        let min_max = self.get_min_max();
        XDimension2D::new(
            f64::from(min_max.width()) * SINGLE_SIZE_X,
            f64::from(min_max.height()) * SINGLE_SIZE_Y,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let min_max = self.get_min_max();
        let x = -f64::from(min_max.min_x) * SINGLE_SIZE_X;
        let y = -f64::from(min_max.min_y) * SINGLE_SIZE_Y;
        let string_bounder = ug.string_bounder();
        for (tile, activity_box) in self.tiles() {
            let position = self.field.get_position(tile);
            let dim_box = activity_box.calculate_dimension(string_bounder);
            let delta_x = SINGLE_SIZE_X * 2.0 - dim_box.width;
            let delta_y = SINGLE_SIZE_Y * 2.0 - dim_box.height;
            activity_box.draw_u(&ug.translated(
                x + f64::from(position.xmin) * SINGLE_SIZE_X + delta_x / 2.0,
                y + f64::from(position.ymin) * SINGLE_SIZE_Y + delta_y / 2.0,
            ));
        }
        let ug = ug.with_color(HColor::MY_RED).with_backcolor(HColor::MY_RED);
        for path in self.field.get_paths() {
            self.draw_path(&ug, (x, y), path);
        }
    }
}

/// A box with rounded corners around its label (PlantUML's `ActivityBox`).
struct ActivityBox {
    id: String,
    tb: SheetBlock2,
}

impl ActivityBox {
    const CORNER: f64 = 25.0;
    const MARGIN: f64 = 10.0;

    fn new(id: &str, label: &str) -> Self {
        let font = FontConfiguration::black_blue_true(UFont::new("Serif", UFontFace::NORMAL, 14));
        let tb = Display::create([label]).create0(
            &font,
            HorizontalAlignment::Left,
            &SpriteContainerEmpty,
            0.0,
            CreoleMode::Full,
        );
        Self {
            id: id.to_owned(),
            tb,
        }
    }
}

impl TextBlock for ActivityBox {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.tb
            .calculate_dimension(string_bounder)
            .delta(2.0 * Self::MARGIN, 2.0 * Self::MARGIN)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self.calculate_dimension(ug.string_bounder());
        let rect = URectangle::new(dim_total.width, dim_total.height).rounded(Self::CORNER);
        let ug = ug
            .with_color(HColor::MY_RED)
            .with_backcolor(HColor::MY_YELLOW);
        ug.with_stroke(UStroke::with_thickness(1.5))
            .draw(&UShape::Rectangle(rect));
        self.tb.draw_u(&ug.translated(Self::MARGIN, Self::MARGIN));
    }
}

impl AbstractDiagram for FlowDiagram {}

impl TitledDiagram for FlowDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for FlowDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let drawing = Drawing {
            field: self
                .field
                .as_ref()
                .expect("a flow diagram without lines is an error, and links need a box"),
            tiles_boxes: &self.tiles_boxes,
        };
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::test_support::error_of;

    #[test]
    fn links_find_the_first_box_declared_with_their_id() {
        let source = Rc::new(UmlSource::new(Vec::new(), Vec::new()));
        let mut diagram = FlowDiagramFactory::create_empty_diagram(&source);
        diagram.line_simple(TileGeometry::South, "a", "A").unwrap();
        diagram.line_simple(TileGeometry::East, "a", "B").unwrap();
        diagram.link_simple(TileGeometry::West, "a").unwrap();
        let paths = diagram.field.as_ref().unwrap().get_paths();
        assert_eq!(paths[1].dest.tile, Tile(0));
    }

    #[test]
    fn unknown_ids_and_blocked_links_are_reported_as_java_exceptions() {
        let message = |text| error_of(text).unwrap().message;
        assert!(
            message("@startflow\na \"A\"\ne b\n@endflow")
                .contains(" java.lang.IllegalArgumentException: b ")
        );
        assert!(
            message("@startflow\na \"A\"\ns b \"B\"\nn c \"C\"\n@endflow")
                .contains(" java.lang.IllegalArgumentException ")
        );
    }
}
