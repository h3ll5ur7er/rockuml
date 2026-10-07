//! A rounded box whose name, description and body parts have their own colours, as composite states and
//! states with a differently coloured name are drawn (PlantUML's `RoundedContainer`).

use super::rounded_north::RoundedNorth;
use super::rounded_south::RoundedSouth;
use crate::color::HColor;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

pub(crate) struct RoundedContainer {
    pub dim: XDimension2D,
    pub name_height: f64,
    pub description_height: f64,
    pub border_color: HColor,
    pub north_backcolor: HColor,
    pub center_backcolor: HColor,
    pub south_backcolor: HColor,
    pub stroke: UStroke,
    pub rounded: f64,
}

impl RoundedContainer {
    pub(crate) fn draw_u(&self, ug: &UGraphic) {
        let ug = ug
            .with_color(self.border_color.clone())
            .with_stroke(self.stroke);
        let width = self.dim.width;
        let rect = UShape::Rectangle(URectangle::new(width, self.dim.height).rounded(self.rounded));
        RoundedNorth {
            width,
            height: self.name_height,
            back_color: self.north_backcolor.clone(),
            rounded: self.rounded,
        }
        .draw_u(&ug);
        self.draw_center(&ug);
        let south_top = self.name_height + self.description_height;
        RoundedSouth {
            width,
            height: self.dim.height - south_top,
            back_color: self.south_backcolor.clone(),
            rounded: self.rounded,
        }
        .draw_u(&ug.translated(0.0, south_top));
        ug.with_backcolor(HColor::NONE).draw(&rect);
        if self.name_height > 0.0 {
            ug.translated(0.0, self.name_height)
                .draw(&UShape::Line { dx: width, dy: 0.0 });
        }
        if self.description_height > 0.0 && south_top > 0.0 {
            ug.translated(0.0, south_top)
                .draw(&UShape::Line { dx: width, dy: 0.0 });
        }
    }

    fn draw_center(&self, ug: &UGraphic) {
        if self.description_height == 0.0 {
            return;
        }
        ug.with_stroke(UStroke::SIMPLE)
            .with_color(self.center_backcolor.clone())
            .with_backcolor(self.center_backcolor.clone())
            .translated(0.0, self.name_height)
            .draw(&UShape::Rectangle(URectangle::new(
                self.dim.width,
                self.description_height,
            )));
    }
}
