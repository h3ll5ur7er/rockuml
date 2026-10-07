use crate::color::HColor;
use crate::klimt::font::StringBounder;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::{Area, Component};

/// A participant's lifeline, with an invisible strip around it that shows the participant's name on hover.
pub(crate) struct ComponentRoseLine {
    color: HColor,
    stroke: UStroke,
    tooltip: String,
}

impl ComponentRoseLine {
    pub(crate) fn new(color: HColor, stroke: UStroke, tooltip: &str) -> Self {
        Self {
            color,
            stroke,
            tooltip: tooltip.to_owned(),
        }
    }
}

impl Component for ComponentRoseLine {
    fn preferred_width(&self, _string_bounder: &dyn StringBounder) -> f64 {
        1.0
    }

    fn preferred_height(&self, _string_bounder: &dyn StringBounder) -> f64 {
        20.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dimension = area.dimension;
        let ug = ug.with_color(self.color.clone()).with_stroke(self.stroke);
        ug.start_group(&UGroup::singleton(UGroupType::Title, &self.tooltip));
        if dimension.height > 0.0 {
            const HOVER_TARGET_WIDTH: f64 = 8.0;
            ug.with_stroke(UStroke::with_thickness(0.0))
                .with_color(HColor::NONE)
                .with_backcolor(HColor::TransparentFill)
                .translated((dimension.width - HOVER_TARGET_WIDTH) / 2.0, 0.0)
                .draw(&UShape::Rectangle(URectangle::new(
                    HOVER_TARGET_WIDTH,
                    dimension.height,
                )));
        }
        let x = (dimension.width / 2.0).trunc();
        ug.translated(x, 0.0).draw(&UShape::Line {
            dx: 0.0,
            dy: dimension.height,
        });
        ug.close_group();
    }
}
