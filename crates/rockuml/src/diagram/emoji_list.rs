//! `emoji`: the emoji of a Unicode block, three columns of codes, pictures and shortcuts (PlantUML's
//! `PSystemListEmojiFactory` and `PSystemListEmoji`).

use std::rc::Rc;

use super::ug_diagram::UgDiagram;
use super::{Diagram, UmlSource};
use crate::emoji::Emoji;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::graphic_strings::GraphicStrings;
use crate::klimt::ugraphic::UGraphic;

/// Each column is this wide, whatever it holds.
const COLUMN_WIDTH: f64 = 500.0;

/// PlantUML's `PSystemListEmojiFactory`: `emoji`, or `emoji` and the start of the codes to list.
pub(super) fn execute_line(source: &Rc<UmlSource>, line: &str) -> Option<Box<dyn Diagram>> {
    let line_lower = line.to_lowercase();
    if line_lower != "emoji" && !line_lower.starts_with("emoji ") {
        return None;
    }
    let code = line.find(' ').map_or("26", |idx| &line[idx + 1..]);
    Some(UgDiagram::create(
        source,
        PSystemListEmoji::new(&code.to_lowercase()),
    ))
}

/// The emoji whose names start with a text.
struct PSystemListEmoji {
    header: GraphicStrings,
    items: Vec<GraphicStrings>,
}

impl PSystemListEmoji {
    fn new(text: &str) -> Self {
        let header = GraphicStrings::create_black_on_white([
            format!("<b><size:16>Emoji available on Unicode Block {text}"),
            "(Blocks available: 26, 27, 1F3, 1F4, 1F5, 1F6, 1F9)".to_owned(),
        ]);
        let items = Emoji::get_all()
            .filter(|(code, _)| code.starts_with(text))
            .map(|(code, emoji)| {
                let shortcut = emoji
                    .shortcut()
                    .map(|shortcut| format!(" \"\"<U+003C>:{shortcut}:<U+003E> \"\""))
                    .unwrap_or_default();
                GraphicStrings::create_black_on_white([format!(
                    "<size:13>\"\"<U+003C>:{code}:<U+003E> \"\"<:{code}:> <#0:{code}:>{shortcut}"
                )])
            })
            .collect();
        Self { header, items }
    }

    /// How many items the first two columns hold.
    fn third(&self) -> usize {
        self.items.len().div_ceil(3)
    }
}

impl TextBlock for PSystemListEmoji {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let header_height = self.header.calculate_dimension(string_bounder).height;
        let third = self.third();
        let mut columns = [0.0; 3];
        for (i, item) in self.items.iter().enumerate() {
            columns[(i / third).min(2)] += item.calculate_dimension(string_bounder).height;
        }
        let max_col_height = columns[0].max(columns[1].max(columns[2]));
        XDimension2D::new(3.0 * COLUMN_WIDTH, header_height + max_col_height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.header.draw_u(ug);
        let string_bounder = ug.string_bounder();
        let top = ug.translated(0.0, self.header.calculate_dimension(string_bounder).height);
        let third = self.third();
        let mut ug = top.clone();
        for (i, item) in self.items.iter().enumerate() {
            item.draw_u(&ug);
            ug = ug.translated(0.0, item.calculate_dimension(string_bounder).height);
            if i + 1 == third {
                ug = top.translated(COLUMN_WIDTH, 0.0);
            }
            if i + 1 == 2 * third {
                ug = top.translated(2.0 * COLUMN_WIDTH, 0.0);
            }
        }
    }
}
