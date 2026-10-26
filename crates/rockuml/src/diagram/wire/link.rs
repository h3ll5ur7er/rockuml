//! Wires from a block to the side of another: thin arrows and wide buses, across or down.

use crate::color::HColor;
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;

/// A thin wire or a wide bus (PlantUML's `WLinkType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WLinkType {
    Normal,
    Bus,
}

impl WLinkType {
    /// `-` makes a wire, `=` a bus.
    pub(super) fn from(style: &str) -> Self {
        if style.contains('-') {
            Self::Normal
        } else {
            Self::Bus
        }
    }

    /// How far the next link from the same block starts.
    pub(super) fn space_for_next(self) -> f64 {
        match self {
            Self::Normal => 15.0,
            Self::Bus => 25.0,
        }
    }
}

/// Which ends have an arrowhead (PlantUML's `WArrowDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WArrowDirection {
    Normal,
    Reverse,
    Both,
    None,
}

impl WArrowDirection {
    pub(super) fn from(style: &str) -> Self {
        match (style.contains('<'), style.contains('>')) {
            (true, true) => Self::Both,
            (false, true) => Self::Normal,
            (true, false) => Self::Reverse,
            (false, false) => Self::None,
        }
    }

    fn points_forward(self) -> bool {
        matches!(self, Self::Normal | Self::Both)
    }

    fn points_backward(self) -> bool {
        matches!(self, Self::Reverse | Self::Both)
    }
}

/// A doubled `-` or `=` runs the link down, a single one across (PlantUML's `WOrientation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WOrientation {
    Horizontal,
    Vertical,
}

impl WOrientation {
    pub(super) fn from(style: &str) -> Self {
        if style.contains("==") || style.contains("--") {
            Self::Vertical
        } else {
            Self::Horizontal
        }
    }
}

fn path(points: &[(f64, f64)]) -> UShape {
    let (first, rest) = points.split_first().expect("a path has a start");
    let mut segments = vec![USegment::MoveTo(first.0, first.1)];
    segments.extend(rest.iter().map(|&(x, y)| USegment::LineTo(x, y)));
    UShape::path(segments)
}

/// A link across, from `start` to the left side of its target at `destination` (PlantUML's
/// `WLinkHorizontal`).
pub(super) struct WLinkHorizontal {
    pub(super) start: UTranslate,
    pub(super) destination: f64,
    pub(super) link_type: WLinkType,
    pub(super) direction: WArrowDirection,
    pub(super) color: HColor,
    pub(super) label: Display,
}

impl WLinkHorizontal {
    fn get_text_block(&self, skin: &SkinParam) -> SheetBlock2 {
        let font =
            FontConfiguration::black_blue_true(UFont::new("SansSerif", UFontFace::NORMAL, 10))
                .with_color(self.color.clone());
        self.label.create0(
            &font,
            HorizontalAlignment::Left,
            skin,
            0.0,
            CreoleMode::Full,
        )
    }

    pub(super) fn draw_me(&self, ug: &UGraphic, skin: &SkinParam) {
        let ug = ug.with_color(self.color.clone());
        let text_block = self.get_text_block(skin);
        let dim_text = text_block.calculate_dimension(ug.string_bounder());
        let mut ug_text = ug.apply(self.start);
        let len = self.destination - self.start.dx;
        match self.link_type {
            WLinkType::Normal => {
                self.draw_normal_arrow(&ug.with_backcolor(self.color.clone()));
                ug_text = ug_text.translated(0.0, -dim_text.height / 2.0);
            }
            WLinkType::Bus => {
                self.draw_bus_arrow(&ug.with_backcolor(HColor::WHITE));
                ug_text = ug_text.translated(0.0, (20.0 - dim_text.height) / 2.0 - 5.0);
            }
        }
        if dim_text.height > 0.0 {
            let dx = match self.direction {
                WArrowDirection::Normal => 4.0,
                WArrowDirection::Reverse => len - dim_text.width - 4.0,
                WArrowDirection::Both | WArrowDirection::None => (len - dim_text.width) / 2.0,
            };
            ug_text = ug_text.translated(dx, 0.0);
            if self.link_type == WLinkType::Normal {
                ug_text
                    .with_color(HColor::WHITE)
                    .with_backcolor(HColor::WHITE)
                    .draw(&UShape::Rectangle(URectangle::new(
                        dim_text.width,
                        dim_text.height,
                    )));
            }
            text_block.draw_u(&ug_text);
        }
    }

    fn draw_bus_arrow(&self, ug: &UGraphic) {
        let dx = self.destination - self.start.dx - 2.0;
        let points: &[(f64, f64)] = match self.direction {
            WArrowDirection::None => &[(0.0, 0.0), (dx, 0.0), (dx, 10.0), (0.0, 10.0), (0.0, 0.0)],
            WArrowDirection::Normal => &[
                (0.0, 0.0),
                (dx - 15.0, 0.0),
                (dx - 15.0, -5.0),
                (dx, 5.0),
                (dx - 15.0, 15.0),
                (dx - 15.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
            ],
            WArrowDirection::Both => &[
                (0.0, 5.0),
                (15.0, -5.0),
                (15.0, 0.0),
                (dx - 15.0, 0.0),
                (dx - 15.0, -5.0),
                (dx, 5.0),
                (dx - 15.0, 15.0),
                (dx - 15.0, 10.0),
                (15.0, 10.0),
                (15.0, 15.0),
                (0.0, 5.0),
            ],
            WArrowDirection::Reverse => &[
                (0.0, 5.0),
                (15.0, -5.0),
                (15.0, 0.0),
                (dx, 0.0),
                (dx, 10.0),
                (15.0, 10.0),
                (15.0, 15.0),
                (0.0, 5.0),
            ],
        };
        ug.apply(self.start.compose(UTranslate::new(1.0, 0.0)))
            .draw(&path(points));
    }

    fn draw_normal_arrow(&self, ug: &UGraphic) {
        let dx = self.destination - self.start.dx - 2.0;
        if self.direction.points_forward() {
            ug.apply(self.start.compose(UTranslate::new(dx, 0.0)))
                .draw(&path(&[(0.0, 0.0), (-5.0, -5.0), (-5.0, 5.0), (0.0, 0.0)]));
        }
        let from = ug.apply(self.start.compose(UTranslate::new(1.0, 0.0)));
        if self.direction.points_backward() {
            from.draw(&path(&[(0.0, 0.0), (5.0, -5.0), (5.0, 5.0), (0.0, 0.0)]));
        }
        from.draw(&UShape::Line { dx, dy: 0.0 });
    }
}

/// A link down, from `start` to the top side of its target at `destination`; it draws no label (PlantUML's
/// `WLinkVertical`).
pub(super) struct WLinkVertical {
    pub(super) start: UTranslate,
    pub(super) destination: f64,
    pub(super) link_type: WLinkType,
    pub(super) direction: WArrowDirection,
    pub(super) color: HColor,
}

impl WLinkVertical {
    pub(super) fn draw_me(&self, ug: &UGraphic) {
        let ug = ug.with_color(self.color.clone());
        match self.link_type {
            WLinkType::Normal => self.draw_normal_arrow(&ug.with_backcolor(self.color.clone())),
            WLinkType::Bus => self.draw_bus_arrow(&ug.with_backcolor(HColor::WHITE)),
        }
    }

    fn draw_bus_arrow(&self, ug: &UGraphic) {
        let dy = self.destination - self.start.dy - 2.0;
        let points: &[(f64, f64)] = match self.direction {
            WArrowDirection::None => &[(0.0, 0.0), (0.0, dy), (10.0, dy), (10.0, 0.0), (0.0, 0.0)],
            WArrowDirection::Normal => &[
                (0.0, 0.0),
                (0.0, dy - 15.0),
                (-5.0, dy - 15.0),
                (5.0, dy),
                (15.0, dy - 15.0),
                (10.0, dy - 15.0),
                (10.0, 0.0),
                (0.0, 0.0),
            ],
            WArrowDirection::Both => &[
                (5.0, 0.0),
                (-5.0, 15.0),
                (0.0, 15.0),
                (0.0, dy - 15.0),
                (-5.0, dy - 15.0),
                (5.0, dy),
                (15.0, dy - 15.0),
                (10.0, dy - 15.0),
                (10.0, 15.0),
                (15.0, 15.0),
                (5.0, 0.0),
            ],
            WArrowDirection::Reverse => &[
                (5.0, 0.0),
                (-5.0, 15.0),
                (0.0, 15.0),
                (0.0, dy),
                (10.0, dy),
                (10.0, 15.0),
                (15.0, 15.0),
                (5.0, 0.0),
            ],
        };
        ug.apply(self.start.compose(UTranslate::new(0.0, 1.0)))
            .draw(&path(points));
    }

    fn draw_normal_arrow(&self, ug: &UGraphic) {
        let dy = self.destination - self.start.dy - 2.0;
        if self.direction.points_forward() {
            ug.apply(self.start.compose(UTranslate::new(0.0, dy)))
                .draw(&path(&[(0.0, 0.0), (5.0, -5.0), (-5.0, -5.0), (0.0, 0.0)]));
        }
        let from = ug.apply(self.start.compose(UTranslate::new(0.0, 1.0)));
        if self.direction.points_backward() {
            from.draw(&path(&[(0.0, 0.0), (5.0, 5.0), (-5.0, 5.0), (0.0, 0.0)]));
        }
        from.draw(&UShape::Line { dx: 0.0, dy });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_tell_type_direction_and_orientation() {
        let read = |style| {
            (
                WLinkType::from(style),
                WArrowDirection::from(style),
                WOrientation::from(style),
            )
        };
        assert_eq!(
            read("->"),
            (
                WLinkType::Normal,
                WArrowDirection::Normal,
                WOrientation::Horizontal
            )
        );
        assert_eq!(
            read("<=="),
            (
                WLinkType::Bus,
                WArrowDirection::Reverse,
                WOrientation::Vertical
            )
        );
        assert_eq!(
            read("<-=>"),
            (
                WLinkType::Normal,
                WArrowDirection::Both,
                WOrientation::Horizontal
            )
        );
        assert_eq!(
            read("="),
            (
                WLinkType::Bus,
                WArrowDirection::None,
                WOrientation::Horizontal
            )
        );
    }
}
