//! `openiconic name` draws an Open Iconic icon on its own; `listopeniconic` lists them all (PlantUML's
//! `openiconic` package).

use std::rc::Rc;

use super::donors::get_cols;
use super::ug_diagram::UgDiagram;
use super::{Diagram, NotYetPorted, UmlSource};
use crate::color::HColor;
use crate::klimt::blocks::TextBlockHorizontal;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{TextBlock, VerticalAlignment};
use crate::openiconic::OpenIconic;

/// PlantUML's `PSystemOpenIconicFactory`: `openiconic` and the icon's name.
pub(super) fn execute_line(
    source: &Rc<UmlSource>,
    line: &str,
) -> Option<Result<Box<dyn Diagram>, NotYetPorted>> {
    let line_lower = line.to_lowercase();
    if !line_lower.starts_with("openiconic ") {
        return None;
    }
    let icon_name = &line_lower["openiconic ".len()..];
    // PlantUML crashes on a name it has no icon of.
    Some(
        OpenIconic::retrieve(icon_name)
            .map(|icon| {
                UgDiagram::with_margin(
                    source,
                    PSystemOpenIconic { icon },
                    ClockwiseTopRightBottomLeft::same(5.0),
                )
            })
            .ok_or(NotYetPorted(
                "the crash report of an unknown Open Iconic icon",
            )),
    )
}

/// The icon in black, at its own size.
struct PSystemOpenIconic {
    icon: OpenIconic,
}

impl TextBlock for PSystemOpenIconic {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.icon
            .as_text_block(HColor::BLACK, 1.0)
            .calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.icon.as_text_block(HColor::BLACK, 1.0).draw_u(ug);
    }
}

/// PlantUML's `PSystemListOpenIconicFactory`: a line that starts with `listopeniconic`.
pub(super) fn execute_list_line(source: &Rc<UmlSource>, line: &str) -> Option<Box<dyn Diagram>> {
    if !line.to_lowercase().starts_with("listopeniconic") {
        return None;
    }
    Some(UgDiagram::create(source, list_open_iconic()))
}

/// Every icon beside its name, in seven columns (PlantUML's `PSystemListOpenIconic`).
fn list_open_iconic() -> TextBlockHorizontal<'static> {
    let mut lines: Vec<String> = [
        "<b>List Open Iconic",
        "<i>Credit to",
        "https://useiconic.com/open",
        " ",
    ]
    .map(str::to_owned)
    .into();
    lines.extend(OpenIconic::names().map(|s| format!("<&{s}> {s}")));
    TextBlockHorizontal::new(get_cols(&lines, 7, 0), VerticalAlignment::Top)
}
