//! Sprite lists: `listsprites` alone lists PlantUML's built-in sprites, those of `ArchiMate`; after sprite
//! definitions it draws the sprites the source defines (PlantUML's `PSystemListArchimateSprites` and `ListSpriteDiagram`).

use std::collections::BTreeSet;
use std::rc::Rc;

use super::builder::CommandFactory;
use super::common_commands::{
    add_common_commands1, add_common_commands2, add_common_scale_commands,
};
use super::diagram_type::DiagramType;
use super::donors::get_cols;
use super::titled::{Titled, TitledDiagram};
use super::ug_diagram::UgDiagram;
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass, PatternCommand, SingleLine};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::blocks::{TextBlockHorizontal, TextBlockVertical};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::sprite::{Sprite, SpriteContainer, SpriteImage};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock, VerticalAlignment};
use crate::pattern::{RegexResult, RegexTree};
use crate::style::SName;
use crate::text::LineLocation;

/// PlantUML's `PSystemListArchimateSpritesFactory`: a line that starts with `listsprite`.
pub(super) fn execute_line(source: &Rc<UmlSource>, line: &str) -> Option<Box<dyn Diagram>> {
    if !line.to_lowercase().starts_with("listsprite") {
        return None;
    }
    Some(UgDiagram::create(source, list_archimate_sprites()))
}

/// Every built-in sprite beside its name, folder by folder in four columns (`PSystemListArchimateSprites`).
fn list_archimate_sprites() -> TextBlockHorizontal<'static> {
    let mut lines: Vec<String> = [
        "<b>List Current Sprites",
        "<i>Credit to",
        "http://www.archimatetool.com",
        " ",
    ]
    .map(str::to_owned)
    .into();
    let paths = SpriteImage::internal_paths();
    let folders: BTreeSet<&str> = paths
        .iter()
        .filter_map(|path| Some(path.split_once('/')?.0))
        .collect();
    for folder in folders {
        lines.push(format!("<u>{folder}</u> :"));
        lines.push(" ".to_owned());
        let files = paths
            .iter()
            .filter_map(|path| path.strip_prefix(folder)?.strip_prefix('/'));
        for file in files {
            let Some((sprite_name, format @ ("png" | "svg"))) = file.rsplit_once('.') else {
                continue;
            };
            lines.push(format!(
                "<$archimate/{sprite_name}> {sprite_name} ({format})"
            ));
        }
    }
    TextBlockHorizontal::new(get_cols(&lines, 4, 0), VerticalAlignment::Top)
}

/// Reads sprite lists (PlantUML's `ListSpriteDiagramFactory`).
pub(super) struct ListSpriteDiagramFactory;

impl CommandFactory for ListSpriteDiagramFactory {
    type Diagram = ListSpriteDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Sprites;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> ListSpriteDiagram {
        ListSpriteDiagram {
            source: source.clone(),
            titled: Titled::new(SName::ActivityDiagram, "HELP", source),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<ListSpriteDiagram>>> {
        let mut commands = add_common_commands1();
        commands.extend(add_common_commands2());
        commands.extend(add_common_scale_commands());
        commands.push(command_list_sprite());
        commands
    }
}

/// PlantUML's `CommandListSprite`: `listsprites` is accepted, the diagram lists its sprites anyway.
fn command_list_sprite() -> Box<dyn Command<ListSpriteDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"listsprites?"),
            RegexTree::end(),
        ]),
        |_: &mut ListSpriteDiagram, _: &LineLocation, _: &RegexResult| Ok(()),
    )))
}

/// The sprites a source defines, in rows of about 1024 pixels (PlantUML's `ListSpriteDiagram`).
pub(super) struct ListSpriteDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
}

impl AbstractDiagram for ListSpriteDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for ListSpriteDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for ListSpriteDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let skin = &self.titled.skin;
        let font =
            FontConfiguration::black_blue_true(UFont::new("SansSerif", UFontFace::NORMAL, 14));
        let sprites = skin
            .get_all_sprite_names()
            .into_iter()
            .filter_map(|name| {
                let block_name = Display::create([name]).create0(
                    &font,
                    HorizontalAlignment::Left,
                    skin,
                    0.0,
                    CreoleMode::Full,
                );
                Some((skin.get_sprite(name)?, block_name))
            })
            .collect();
        Ok(self
            .titled
            .add_chrome(Box::new(SpriteRows { sprites }), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

/// Each sprite above its name.
struct SpriteRows {
    sprites: Vec<(Rc<dyn Sprite>, SheetBlock2)>,
}

impl SpriteRows {
    /// Lays the sprites out, drawing them if there is `ug` to draw on.
    fn layout(&self, string_bounder: &dyn StringBounder, ug: Option<&UGraphic>) -> XDimension2D {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut raw_height: f64 = 0.0;
        let mut total_width: f64 = 0.0;
        for (sprite, block_name) in &self.sprites {
            let tb = TextBlockVertical::new(
                vec![
                    sprite.as_text_block(&HColor::BLACK, None, 1.0, None),
                    Box::new(block_name),
                ],
                HorizontalAlignment::Center,
            );
            if let Some(ug) = ug {
                tb.draw_u(&ug.translated(x, y));
            }
            let dim = tb.calculate_dimension(string_bounder);
            raw_height = raw_height.max(dim.height);
            x += dim.width;
            x += 30.0;
            total_width = total_width.max(x);
            if x > 1024.0 {
                x = 0.0;
                y += raw_height + 50.0;
                raw_height = 0.0;
            }
        }
        XDimension2D::new(total_width, y + raw_height)
    }
}

impl TextBlock for SpriteRows {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.layout(string_bounder, None)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.layout(ug.string_bounder(), Some(ug));
    }
}
