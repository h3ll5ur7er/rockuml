//! The circles the flow starts and ends at, and the lettered connector circle (PlantUML's
//! `FtileCircleStart`, `FtileCircleStop`, `FtileCircleEndCross` and `FtileCircleSpot`).

use std::rc::Rc;

use crate::color::{Colors, HColor};
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::klimt::TextBlock;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::{UCenteredCharacter, UEllipse, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};
use crate::svek::image::{CircleEnd, CircleStart};

/// The lane of a circle, which lies in that lane alone.
macro_rules! mono_swimable {
    ($tile:ty) => {
        impl Swimable for $tile {
            fn get_swimlanes(&self) -> SwimlaneSet {
                self.swimlane.map(Some).into_iter().collect()
            }

            fn get_swimlane_in(&self) -> Option<SwimlaneId> {
                self.swimlane
            }

            fn get_swimlane_out(&self) -> Option<SwimlaneId> {
                self.swimlane
            }
        }
    };
}

/// `start`: a filled circle.
pub(crate) struct FtileCircleStart {
    base: AbstractFtile,
    circle: CircleStart,
    swimlane: Option<SwimlaneId>,
}

impl FtileCircleStart {
    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        swimlane: Option<SwimlaneId>,
        style: &Style,
        colors: &Colors,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            circle: CircleStart::new(style, colors),
            swimlane,
        }
    }
}

mono_swimable!(FtileCircleStart);

impl Ftile for FtileCircleStart {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let size = self.circle.calculate_dimension(string_bounder).width;
            FtileGeometry::with_out(size, size, size / 2.0, 0.0, size)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.circle.draw_u(ug);
    }
}

/// `stop`: a ringed circle, no arrow leaving it.
pub(crate) struct FtileCircleStop {
    base: AbstractFtile,
    circle: CircleEnd,
    swimlane: Option<SwimlaneId>,
}

impl FtileCircleStop {
    const SIZE: f64 = 22.0;

    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        swimlane: Option<SwimlaneId>,
        style: &Style,
        colors: &Colors,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            circle: CircleEnd::new(style, colors),
            swimlane,
        }
    }
}

mono_swimable!(FtileCircleStop);

impl Ftile for FtileCircleStop {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            FtileGeometry::new(Self::SIZE, Self::SIZE, Self::SIZE / 2.0, 0.0)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.circle.draw_u(ug);
    }
}

/// `end`: a crossed circle, no arrow leaving it.
pub(crate) struct FtileCircleEndCross {
    base: AbstractFtile,
    line_color: HColor,
    back_color: HColor,
    swimlane: Option<SwimlaneId>,
    stroke: UStroke,
}

impl FtileCircleEndCross {
    const SIZE: f64 = 20.0;

    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        swimlane: Option<SwimlaneId>,
        style: &Style,
        colors: &Colors,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            line_color: colors.get_color_of(style, PName::LineColor),
            back_color: colors.get_color_of(style, PName::BackGroundColor),
            swimlane,
            stroke: style.stroke(),
        }
    }
}

mono_swimable!(FtileCircleEndCross);

impl Ftile for FtileCircleEndCross {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            FtileGeometry::new(Self::SIZE, Self::SIZE, Self::SIZE / 2.0, 0.0)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug
            .apply(self.line_color.clone())
            .with_backcolor(HColor::NONE);
        ug.apply(self.stroke)
            .draw(&UShape::Ellipse(UEllipse::new(Self::SIZE, Self::SIZE)));
        let ug = ug.with_backcolor(self.back_color.clone());
        let thickness = 2.5;
        let size2 = (Self::SIZE - thickness) / 2.0_f64.sqrt();
        let delta = (Self::SIZE - size2) / 2.0;
        let ug = ug.apply(UStroke::with_thickness(thickness));
        ug.apply(UTranslate::new(delta, delta)).draw(&UShape::Line {
            dx: size2,
            dy: size2,
        });
        ug.apply(UTranslate::new(delta, Self::SIZE - delta))
            .draw(&UShape::Line {
                dx: size2,
                dy: -size2,
            });
    }
}

/// `(A)`: a circle with a letter, where arrows meet without a line between them.
pub(crate) struct FtileCircleSpot {
    base: AbstractFtile,
    swimlane: Option<SwimlaneId>,
    spot: String,
    fc: FontConfiguration,
    back_color: Option<HColor>,
    style: Style,
}

impl FtileCircleSpot {
    const SIZE: f64 = 20.0;

    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        swimlane: Option<SwimlaneId>,
        spot: &str,
        back_color: Option<HColor>,
        style: Style,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            swimlane,
            spot: spot.to_owned(),
            fc: style.font_configuration(),
            back_color,
            style,
        }
    }
}

mono_swimable!(FtileCircleSpot);

impl Ftile for FtileCircleSpot {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            FtileGeometry::with_out(Self::SIZE, Self::SIZE, Self::SIZE / 2.0, 0.0, Self::SIZE)
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let back_color = self
            .back_color
            .clone()
            .unwrap_or_else(|| self.style.value(PName::BackGroundColor).as_color());
        let border_color = self.style.value(PName::LineColor).as_color();
        ug.apply(border_color)
            .with_backcolor(back_color)
            .apply(self.style.stroke())
            .draw(&UShape::Ellipse(UEllipse::new(Self::SIZE, Self::SIZE)));
        let Some(character) = self.spot.chars().next() else {
            return;
        };
        ug.apply(self.fc.color().clone())
            .apply(UTranslate::new(Self::SIZE / 2.0, Self::SIZE / 2.0))
            .draw(&UShape::CenteredCharacter(UCenteredCharacter {
                character,
                font: self.fc.font(),
            }));
    }
}
