//! A group's background, in bands that change colour where an `else` starts (PlantUML's `Blotter`).

use crate::color::HColor;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct Blotter {
    dimension: XDimension2D,
    default_backcolor: HColor,
    round: f64,
    last: HColor,
    /// Where each band ends, top to bottom, and the colour of the band after it.
    changes: Vec<(f64, HColor)>,
}

impl Blotter {
    pub(super) fn new(dimension: XDimension2D, default_backcolor: HColor, round: f64) -> Self {
        Self {
            dimension,
            last: default_backcolor.clone(),
            default_backcolor,
            round,
            changes: Vec::new(),
        }
    }

    /// A new band from `y` on, unless it keeps the colour of the band before.
    pub(super) fn add_change(&mut self, y: f64, color: HColor) {
        if color == self.last {
            return;
        }
        self.put(y, color.clone());
        self.last = color;
    }

    pub(super) fn close_changes(&mut self) {
        self.put(self.dimension.height, self.default_backcolor.clone());
    }

    fn put(&mut self, y: f64, color: HColor) {
        match self.changes.binary_search_by(|(key, _)| key.total_cmp(&y)) {
            Ok(index) => self.changes[index].1 = color,
            Err(index) => self.changes.insert(index, (y, color)),
        }
    }

    pub(super) fn draw_u(&self, ug: &UGraphic) {
        let mut current = &self.default_backcolor;
        let mut y = 0.0;
        for (i, (end, color)) in self.changes.iter().enumerate() {
            if !current.is_transparent() {
                ug.with_color(current.clone())
                    .with_backcolor(current.clone())
                    .translated(0.0, y)
                    .draw(&self.rectangle_background(i, end - y));
            }
            y = *end;
            current = color;
        }
    }

    /// The first and last bands take the rounded corners.
    fn rectangle_background(&self, i: usize, height: f64) -> UShape {
        let width = self.dimension.width;
        let round = self.round;
        let rectangle = URectangle::new(width, height);
        if round == 0.0 {
            return UShape::Rectangle(rectangle);
        }
        if self.changes.len() == 1 {
            return UShape::Rectangle(rectangle.rounded(round));
        }
        let half = round / 2.0;
        let arc = |end| USegment::ArcTo {
            radius: (half, half),
            x_axis_rotation: 0.0,
            large_arc: false,
            sweep: true,
            end,
        };
        if i == 0 {
            return UShape::path(vec![
                USegment::MoveTo(half, 0.0),
                USegment::LineTo(width - half, 0.0),
                arc((width, half)),
                USegment::LineTo(width, height),
                USegment::LineTo(0.0, height),
                USegment::LineTo(0.0, half),
                arc((half, 0.0)),
            ]);
        }
        if i == self.changes.len() - 1 {
            return UShape::path(vec![
                USegment::MoveTo(0.0, 0.0),
                USegment::LineTo(width, 0.0),
                USegment::LineTo(width, height - half),
                arc((width - half, height)),
                USegment::LineTo(half, height),
                arc((0.0, height - half)),
                USegment::LineTo(0.0, 0.0),
            ]);
        }
        UShape::Rectangle(rectangle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pink() -> HColor {
        HColor::parse("pink").unwrap().unwrap()
    }

    #[test]
    fn a_band_in_the_colour_before_starts_nothing() {
        let mut blotter = Blotter::new(XDimension2D::new(100.0, 50.0), HColor::NONE, 0.0);
        blotter.add_change(10.0, HColor::NONE);
        blotter.add_change(20.0, pink());
        blotter.add_change(30.0, pink());
        blotter.close_changes();
        assert_eq!(blotter.changes, vec![(20.0, pink()), (50.0, HColor::NONE)]);
    }

    #[test]
    fn the_last_change_at_a_place_wins() {
        let mut blotter = Blotter::new(XDimension2D::new(100.0, 50.0), HColor::NONE, 0.0);
        blotter.add_change(50.0, pink());
        blotter.close_changes();
        assert_eq!(blotter.changes, vec![(50.0, HColor::NONE)]);
    }
}
