use std::cell::RefCell;
use std::rc::Rc;

use super::CompressionMode;
use super::slot::SlotSet;
use crate::color::HColor;
use crate::klimt::clip::path_bounds;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::{UPath, URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UGraphicBackend, UParam};

/// A surface recording where along one axis shapes are drawn (`SlotFinder`). Lines and centred titles take
/// no room: what they cross can be squeezed.
pub(crate) struct SlotFinder {
    mode: CompressionMode,
    string_bounder: Rc<dyn StringBounder>,
    slot: SlotSet,
}

impl SlotFinder {
    /// A surface finding the slots taken along `mode`'s axis.
    pub(crate) fn create(
        mode: CompressionMode,
        string_bounder: Rc<dyn StringBounder>,
    ) -> (UGraphic, Rc<RefCell<SlotFinder>>) {
        let finder = Rc::new(RefCell::new(SlotFinder {
            mode,
            string_bounder: string_bounder.clone(),
            slot: SlotSet::default(),
        }));
        // PlantUML's measuring surfaces (`UGraphicNo`) answer black.
        let ug = UGraphic::new(finder.clone(), string_bounder, HColor::BLACK);
        (ug, finder)
    }

    pub(crate) fn get_slot_set(&self) -> &SlotSet {
        &self.slot
    }

    /// Records `x1..x2` when compressing across, `y1..y2` when compressing down.
    fn add_slot(&mut self, (x1, x2): (f64, f64), (y1, y2): (f64, f64)) {
        match self.mode {
            CompressionMode::OnX => self.slot.add_slot(x1, x2),
            CompressionMode::OnY => self.slot.add_slot(y1, y2),
        }
    }

    /// A frame ignored on this axis only takes room at its two borders (`drawWhenCompressed`).
    fn draw_when_compressed(&mut self, x: f64, y: f64, rectangle: &URectangle) {
        match self.mode {
            CompressionMode::OnX => {
                let right = x + (rectangle.width - 2.0);
                self.slot.add_slot(x, x + 2.0);
                self.slot.add_slot(right, right + 2.0);
            }
            CompressionMode::OnY => {
                let bottom = y + (rectangle.height - 2.0);
                self.slot.add_slot(y, y + 2.0);
                self.slot.add_slot(bottom, bottom + 2.0);
            }
        }
    }
}

impl UGraphicBackend for SlotFinder {
    fn draw(&mut self, shape: &UShape, at: UTranslate, _param: &UParam) {
        let (x, y) = (at.dx, at.dy);
        match shape {
            UShape::Rectangle(rectangle) if rectangle.is_ignore_for_compression_on(self.mode) => {
                self.draw_when_compressed(x, y, rectangle);
            }
            UShape::Rectangle(rectangle) => {
                self.add_slot((x, x + rectangle.width), (y, y + rectangle.height));
            }
            UShape::Path(path) if path.is_ignore_for_compression_on(self.mode) => {}
            UShape::Path(UPath { segments, .. }) => {
                if let Some((min_x, min_y, max_x, max_y)) = path_bounds(segments) {
                    self.add_slot((x + min_x, x + max_x), (y + min_y, y + max_y));
                }
            }
            UShape::Polygon(polygon) => {
                if polygon.get_compression_mode() != Some(self.mode) {
                    let bounds = polygon.min_max();
                    self.add_slot(
                        (x + bounds.min_x(), x + bounds.max_x()),
                        (y + bounds.min_y(), y + bounds.max_y()),
                    );
                }
            }
            UShape::Ellipse(ellipse) => {
                self.add_slot((x, x + ellipse.width), (y, y + ellipse.height));
            }
            UShape::Text(text) => {
                let dimension = self
                    .string_bounder
                    .calculate_dimension(&text.font.font(), &text.text);
                let top = y - (dimension.height - 1.5);
                self.add_slot((x, x + dimension.width), (top, top + dimension.height));
            }
            UShape::Empty(dimension) => {
                self.add_slot((x, x + dimension.width), (y, y + dimension.height));
            }
            UShape::Line { .. }
            | UShape::Image(_)
            | UShape::ImageSvg(_)
            | UShape::CenteredCharacter(_)
            | UShape::HorizontalLine
            | UShape::SpecialText
            | UShape::Comment(_)
            | UShape::CenteredText(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::debug::StringBounderDebug;
    use crate::klimt::shape::USegment;

    fn slots(mode: CompressionMode) -> Vec<(f64, f64)> {
        let (ug, finder) = SlotFinder::create(mode, Rc::new(StringBounderDebug));
        let tab = UPath::new(vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::LineTo(50.0, 20.0),
        ]);
        ug.draw(&UShape::Path(tab.ignore_for_compression_on_x()));
        ug.translated(60.0, 30.0)
            .draw(&UShape::Rectangle(URectangle::new(10.0, 10.0)));
        let finder = finder.borrow();
        let mut slots: Vec<(f64, f64)> = finder
            .get_slot_set()
            .get_slots()
            .iter()
            .map(|slot| (slot.get_start(), slot.get_end()))
            .collect();
        slots.sort_by(|a, b| a.0.total_cmp(&b.0));
        slots
    }

    #[test]
    fn a_frame_title_tab_takes_room_down_but_not_across() {
        assert_eq!(slots(CompressionMode::OnX), [(60.0, 70.0)]);
        assert_eq!(slots(CompressionMode::OnY), [(0.0, 20.0), (30.0, 40.0)]);
    }
}
