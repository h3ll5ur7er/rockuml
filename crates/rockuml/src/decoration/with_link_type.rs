//! The style written in brackets in an arrow, like `-[#red,dashed]->` (PlantUML's `WithLinkType`).

use super::link_type::LinkType;
use crate::color::HColor;

/// An arrow whose line an arrow style changes.
pub(crate) trait WithLinkType {
    fn link_type_mut(&mut self) -> &mut LinkType;

    /// The colour of line `i`: 0 is the arrow itself, the others parallel lines.
    fn set_specific_color(&mut self, color: HColor, i: usize);

    fn go_hidden(&mut self) {}

    fn go_single(&mut self) {}

    fn go_norank(&mut self) {}

    /// `dashed,#red;bold`: `;` separates the styles of parallel lines.
    fn apply_style(&mut self, arrow_style: Option<&str>) {
        let Some(arrow_style) = arrow_style else {
            return;
        };
        for (i, style) in tokens(arrow_style, ';').enumerate() {
            for s in tokens(style, ',') {
                if s.eq_ignore_ascii_case("dashed") {
                    *self.link_type_mut() = self.link_type_mut().go_dashed();
                } else if s.eq_ignore_ascii_case("bold") {
                    *self.link_type_mut() = self.link_type_mut().go_bold();
                } else if s.eq_ignore_ascii_case("dotted") {
                    *self.link_type_mut() = self.link_type_mut().go_dotted();
                } else if s.eq_ignore_ascii_case("hidden") {
                    self.go_hidden();
                } else if s.eq_ignore_ascii_case("single") {
                    self.go_single();
                } else if s.eq_ignore_ascii_case("plain") {
                    // A plain line is the default.
                } else if s.eq_ignore_ascii_case("node") {
                    // Only the state command reads it, from the arrow itself.
                } else if s.eq_ignore_ascii_case("norank") {
                    self.go_norank();
                } else if let Some(thickness) = s.strip_prefix("thickness=") {
                    let thickness = thickness.parse().unwrap_or_default();
                    *self.link_type_mut() = self.link_type_mut().go_thickness(thickness);
                } else {
                    self.set_specific_color(HColor::parse_or_white(s), i);
                }
            }
        }
    }
}

/// `StringTokenizer`: the non-empty pieces between delimiters.
fn tokens(text: &str, delimiter: char) -> impl Iterator<Item = &str> {
    text.split(delimiter).filter(|token| !token.is_empty())
}
