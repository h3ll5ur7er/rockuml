//! Boards, `@startboard`: story maps of cards, an activity on each line without `+` and its releases below
//! it, one `+` per stage (PlantUML's `board` package and `BoardDiagramFactory`).

use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::{HColor, XColor};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass, PatternCommand, SingleLine};
use crate::creole::{CreoleMode, Display};
use crate::java::JavaHashMap;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::pattern::{RegexResult, RegexTree};
use crate::skin::SkinParam;
use crate::style::SName;
use crate::text::LineLocation;

/// The room a card takes on the board, gaps included (`PostIt`).
const POST_IT_WIDTH: f64 = 170.0;
const POST_IT_HEIGHT: f64 = 90.0;

const LIGHT_GRAY: HColor = HColor::Simple(XColor::rgb(0xC0, 0xC0, 0xC0));

/// Reads boards (PlantUML's `BoardDiagramFactory`).
pub(super) struct BoardDiagramFactory;

impl CommandFactory for BoardDiagramFactory {
    type Diagram = BoardDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Board;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> BoardDiagram {
        BoardDiagram {
            source: source.clone(),
            titled: Titled::new(SName::ActivityDiagram, "BOARD", source),
            activities: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<BoardDiagram>>> {
        let mut commands = add_common_commands1();
        commands.push(command_board_plus());
        commands
    }
}

/// PlantUML's `CommandBoardPlus`: a label after as many `+` as its stage, none for an activity.
fn command_board_plus() -> Box<dyn Command<BoardDiagram>> {
    Box::new(SingleLine(
        PatternCommand::new(
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(1, "PLUS", r"([+]*)"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "LABEL", r"([^%s].*)"),
                RegexTree::end(),
            ]),
            |diagram: &mut BoardDiagram, _: &LineLocation, arg: &RegexResult| {
                diagram.add_line(
                    arg.get("PLUS", 0).unwrap_or_default(),
                    arg.get("LABEL", 0).unwrap_or_default(),
                );
                Ok(())
            },
        )
        .untrimmed(),
    ))
}

/// A `@startboard` diagram (PlantUML's `BoardDiagram`).
pub(super) struct BoardDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    activities: Vec<Activity>,
}

impl BoardDiagram {
    fn add_line(&mut self, plus: &str, label: &str) {
        if plus.is_empty() {
            self.activities.push(Activity::new(label));
        } else if let Some(activity) = self.activities.last_mut() {
            activity.add_release(plus.len(), label);
        } else {
            self.titled.not_ported(NotYetPorted(
                "a release before any activity, where PlantUML crashes",
            ));
        }
    }
}

/// A card of the board: an activity at stage 0 or a release below it (`BNode`).
struct BNode {
    name: String,
    stage: usize,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// An activity and its releases, the tree of cards in one or more columns (`Activity`). Node 0 is the
/// activity.
struct Activity {
    nodes: Vec<BNode>,
    cursor: usize,
}

impl Activity {
    fn new(name: &str) -> Self {
        Self {
            nodes: vec![BNode {
                name: name.to_owned(),
                stage: 0,
                parent: None,
                children: Vec::new(),
            }],
            cursor: 0,
        }
    }

    /// Hangs the release below the last card of a lower stage.
    fn add_release(&mut self, stage: usize, label: &str) {
        while stage <= self.nodes[self.cursor].stage {
            self.cursor = self.nodes[self.cursor]
                .parent
                .expect("the activity's stage is below every release's");
        }
        let node = self.nodes.len();
        self.nodes.push(BNode {
            name: label.to_owned(),
            stage,
            parent: Some(self.cursor),
            children: Vec::new(),
        });
        self.nodes[self.cursor].children.push(node);
        self.cursor = node;
    }

    /// Each card's column: a card shares its first child's, and every further child opens a new one
    /// (`BNode.computeX`).
    fn compute_x(&self, node: usize, count: &mut usize, xs: &mut [usize]) {
        xs[node] = *count;
        for (i, &child) in self.nodes[node].children.iter().enumerate() {
            if i > 0 {
                *count += 1;
            }
            self.compute_x(child, count, xs);
        }
    }

    fn get_array(&self) -> BArray {
        let mut xs = vec![0; self.nodes.len()];
        self.compute_x(0, &mut 0, &mut xs);
        let mut array = BArray::default();
        self.init_barray(0, &xs, &mut array);
        array
    }

    fn init_barray(&self, node: usize, xs: &[usize], array: &mut BArray) {
        array.put(xs[node], self.nodes[node].stage, node);
        for &child in &self.nodes[node].children {
            self.init_barray(child, xs, array);
        }
    }

    fn get_full_width(&self) -> f64 {
        (self.get_array().max_x + 1) as f64 * POST_IT_WIDTH
    }

    fn get_max_stage(&self) -> usize {
        self.get_array().max_y
    }

    fn draw_me(&self, ug: &UGraphic, skin: &SkinParam) {
        CardBox::new(&self.nodes[0].name, skin).draw_u(ug);
        for &(x, stage, node) in self.get_array().cells.iter().map(|(_, cell)| cell) {
            let dx = x as f64 * POST_IT_WIDTH;
            let dy = stage as f64 * POST_IT_HEIGHT;
            CardBox::new(&self.nodes[node].name, skin).draw_u(&ug.translated(dx, dy));
        }
    }
}

/// The cards of an activity by column and stage, in the order of PlantUML's `HashMap` of `"x;y"` keys
/// (`BArray`).
#[derive(Default)]
struct BArray {
    cells: JavaHashMap<(usize, usize, usize)>,
    max_x: usize,
    max_y: usize,
}

impl BArray {
    fn put(&mut self, x: usize, stage: usize, node: usize) {
        self.cells.put(format!("{x};{stage}"), (x, stage, node));
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(stage);
    }
}

/// A grey card with its label at the top left (`CardBox`).
struct CardBox<'a> {
    label: &'a str,
    skin: &'a SkinParam,
}

impl<'a> CardBox<'a> {
    fn new(label: &'a str, skin: &'a SkinParam) -> Self {
        Self { label, skin }
    }
}

impl TextBlock for CardBox<'_> {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(150.0, 70.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        let rect = URectangle::new(dim.width, dim.height).with_delta_shadow(1.0);
        ug.with_color(HColor::BLACK)
            .with_backcolor(LIGHT_GRAY)
            .draw(&UShape::Rectangle(rect));
        let font =
            FontConfiguration::black_blue_true(UFont::new("SansSerif", UFontFace::NORMAL, 14));
        Display::create([self.label])
            .create0(
                &font,
                HorizontalAlignment::Left,
                self.skin,
                0.0,
                CreoleMode::Full,
            )
            .draw_u(&ug.translated(3.0, 3.0));
    }
}

/// The activities side by side, dashed lines between the stages.
struct Drawing<'a> {
    diagram: &'a BoardDiagram,
}

impl Drawing<'_> {
    fn get_full_width(&self) -> f64 {
        self.diagram
            .activities
            .iter()
            .map(Activity::get_full_width)
            .sum()
    }

    fn get_max_stage(&self) -> usize {
        self.diagram
            .activities
            .iter()
            .map(Activity::get_max_stage)
            .max()
            .unwrap_or(0)
    }
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        LimitFinder::min_max_from_origin_of(self, string_bounder.shared())
            .dimension()
            .delta(10.0, 10.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let mut mug = ug.clone();
        for activity in &self.diagram.activities {
            activity.draw_me(&mug, &self.diagram.titled.skin);
            mug = mug.translated(activity.get_full_width(), 0.0);
        }
        let line = UShape::Line {
            dx: self.get_full_width(),
            dy: 0.0,
        };
        let dashed = ug.with_color(HColor::BLACK).with_stroke(UStroke {
            dash_visible: 5.0,
            dash_space: 5.0,
            thickness: 0.5,
        });
        for i in 0..self.get_max_stage() {
            let dy = (i + 1) as f64 * POST_IT_HEIGHT - 10.0;
            dashed.translated(0.0, dy).draw(&line);
        }
    }
}

impl AbstractDiagram for BoardDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for BoardDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for BoardDiagram {
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

    fn activity(releases: &[&str]) -> Activity {
        let mut activity = Activity::new("activity");
        for release in releases {
            let label = release.trim_start_matches('+');
            activity.add_release(release.len() - label.len(), label);
        }
        activity
    }

    /// Each card as `name x;stage` in drawing order.
    fn cells(activity: &Activity) -> Vec<String> {
        activity
            .get_array()
            .cells
            .iter()
            .map(|(key, &(_, _, node))| format!("{} {key}", activity.nodes[node].name))
            .collect()
    }

    #[test]
    fn further_children_open_columns_and_skipped_stages_stay_empty() {
        let activity = activity(&["+a", "++b", "++c", "+++d", "+e", "+++f", "++g"]);
        let mut cells = cells(&activity);
        cells.sort();
        assert_eq!(
            cells,
            [
                "a 0;1",
                "activity 0;0",
                "b 0;2",
                "c 1;2",
                "d 1;3",
                "e 2;1",
                "f 2;3",
                "g 3;2"
            ]
        );
        assert_eq!(activity.get_full_width(), 4.0 * POST_IT_WIDTH);
        assert_eq!(activity.get_max_stage(), 3);
    }

    #[test]
    fn cards_come_in_the_order_of_java_s_hash_map() {
        let activity = activity(&["+a", "+b", "++c"]);
        assert_eq!(
            cells(&activity),
            ["activity 0;0", "a 0;1", "b 1;1", "c 1;2"]
        );
    }
}
