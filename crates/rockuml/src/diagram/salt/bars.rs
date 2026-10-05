//! Tab bars (`{/`) and menu bars (`{*`) with their popups.

use super::super::NotYetPorted;
use super::elements::{Element, Text, color, text_block, widget_font};
use crate::color::HColor;
use crate::creole::SheetBlock1;
use crate::klimt::TextBlock;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;

/// Tabs side by side, each framed on top and at its sides and joined to the next at the bottom.
#[derive(Default)]
pub(super) struct TabBar {
    tabs: Vec<Text>,
}

impl TabBar {
    const MARGIN1: f64 = 2.0;
    const MARGIN2: f64 = 3.0;
    const MARGIN3: f64 = 10.0;

    pub(super) fn add_tab(&mut self, tab: &str) {
        self.tabs.push(Text::new(tab, widget_font()));
    }
}

impl Element for TabBar {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.tabs
            .iter()
            .map(|tab| tab.preferred_dimension(string_bounder))
            .fold(XDimension2D::new(0.0, 0.0), |bar, tab| {
                XDimension2D::new(
                    bar.width + (tab.width + Self::MARGIN1 + Self::MARGIN2 + Self::MARGIN3),
                    bar.height.max(tab.height),
                )
            })
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, dimension: XDimension2D) {
        if z_index != 0 {
            return;
        }
        let ug = ug.with_color(HColor::BLACK);
        let mut x = 0.0;
        for tab in &self.tabs {
            tab.draw_u(&ug.translated(x + Self::MARGIN1, 0.0), z_index, dimension);
            let text = tab.preferred_dimension(ug.string_bounder());
            let side = UShape::Line {
                dx: 0.0,
                dy: text.height,
            };
            let right = x + text.width + Self::MARGIN1 + Self::MARGIN2;
            ug.translated(x, 0.0).draw(&side);
            ug.translated(x, 0.0).draw(&UShape::Line {
                dx: text.width + Self::MARGIN1 + Self::MARGIN2,
                dy: 0.0,
            });
            ug.translated(right, 0.0).draw(&side);
            ug.translated(right, text.height).draw(&UShape::Line {
                dx: Self::MARGIN3,
                dy: 0.0,
            });
            x += text.width + Self::MARGIN1 + Self::MARGIN2 + Self::MARGIN3;
        }
    }
}

/// A menu bar whose entries may open a popup, drawn in the second pass over everything else.
#[derive(Default)]
pub(super) struct MenuBar {
    entries: Vec<MenuEntry>,
    /// Each popup with the index of the entry that opens it.
    popups: Vec<(usize, MenuPopup)>,
}

impl MenuBar {
    const GAP: f64 = 10.0;

    pub(super) fn add_entry(&mut self, text: &str) {
        self.entries.push(MenuEntry::new(text));
    }

    /// Adds `sub` to the popup of the entry named `entry`.
    pub(super) fn add_sub_entry(&mut self, entry: &str, sub: &str) -> Result<(), NotYetPorted> {
        let index = self
            .entries
            .iter()
            .position(|candidate| candidate.text == entry)
            .ok_or(NotYetPorted(
                "crash report for a salt popup without its menu",
            ))?;
        let position = self
            .popups
            .iter()
            .position(|(owner, _)| *owner == index)
            .unwrap_or_else(|| {
                self.popups.push((index, MenuPopup::default()));
                self.popups.len() - 1
            });
        self.popups[position].1.entries.push(MenuEntry::new(sub));
        Ok(())
    }

    fn entry_x(&self, index: usize, string_bounder: &dyn StringBounder) -> f64 {
        self.entries[..index].iter().fold(0.0, |x, entry| {
            x + (entry.preferred_dimension(string_bounder).width + Self::GAP)
        })
    }
}

impl Element for MenuBar {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.entries
            .iter()
            .map(|entry| entry.preferred_dimension(string_bounder))
            .fold(XDimension2D::new(0.0, 0.0), |bar, entry| {
                XDimension2D::new(
                    bar.width + (entry.width + Self::GAP),
                    bar.height.max(entry.height),
                )
            })
    }

    fn draw_u(&self, ug: &UGraphic, z_index: i32, dimension: XDimension2D) {
        let ug = ug.with_color(HColor::BLACK);
        let string_bounder = ug.string_bounder();
        match z_index {
            0 => {
                ug.with_backcolor(color("#D"))
                    .draw(&UShape::Rectangle(URectangle::new(
                        dimension.width,
                        dimension.height,
                    )));
                for (index, entry) in self.entries.iter().enumerate() {
                    entry.draw(&ug.translated(self.entry_x(index, string_bounder), 0.0));
                }
            }
            1 => {
                let y = self.preferred_dimension(string_bounder).height;
                for (index, popup) in &self.popups {
                    let x = self.entry_x(*index, string_bounder);
                    popup.draw(
                        &ug.translated(x, y),
                        popup.preferred_dimension(string_bounder),
                    );
                }
            }
            _ => {}
        }
    }
}

/// An entry of a menu bar or popup; `-` separates entries of a popup.
struct MenuEntry {
    block: SheetBlock1,
    text: String,
}

impl MenuEntry {
    fn new(text: &str) -> Self {
        let font = FontConfiguration::black_blue_true(widget_font());
        Self {
            block: text_block(&[text.to_owned()], &font),
            text: text.to_owned(),
        }
    }

    fn is_separator(&self) -> bool {
        self.text == "-"
    }

    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        if self.is_separator() {
            XDimension2D::new(10.0, 5.0)
        } else {
            self.block.calculate_dimension(string_bounder)
        }
    }

    fn draw(&self, ug: &UGraphic) {
        self.block.draw_u(ug);
    }
}

#[derive(Default)]
struct MenuPopup {
    entries: Vec<MenuEntry>,
}

impl MenuPopup {
    fn preferred_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.entries
            .iter()
            .map(|entry| entry.preferred_dimension(string_bounder))
            .fold(XDimension2D::new(0.0, 0.0), |popup, entry| {
                XDimension2D::new(popup.width.max(entry.width), popup.height + entry.height)
            })
    }

    fn draw(&self, ug: &UGraphic, dimension: XDimension2D) {
        ug.with_backcolor(color("#D"))
            .draw(&UShape::Rectangle(URectangle::new(
                dimension.width,
                dimension.height,
            )));
        let mut y = 0.0;
        for entry in &self.entries {
            let height = entry.preferred_dimension(ug.string_bounder()).height;
            if entry.is_separator() {
                ug.translated(0.0, y + height / 2.0).draw(&UShape::Line {
                    dx: dimension.width,
                    dy: 0.0,
                });
            } else {
                entry.draw(&ug.translated(0.0, y));
            }
            y += height;
        }
    }
}
