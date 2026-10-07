//! A state without description under `hide empty description`: its name alone in a rounded box (PlantUML's
//! `EntityImageStateEmptyDescription`).

use super::entity_image_state_common::{EntityImageStateCommon, get_style_state};
use crate::abel::Entity;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;
use crate::style::{SName, Style};
use crate::svek::{IEntityImage, MARGIN};

const MIN_WIDTH: f64 = 50.0;
const MIN_HEIGHT: f64 = 40.0;

pub(crate) struct EntityImageStateEmptyDescription {
    common: EntityImageStateCommon,
    /// PlantUML paints the box with the name's style, which its `getStyleStateDescription` returns.
    style_state_name: Style,
}

impl EntityImageStateEmptyDescription {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self {
            common: EntityImageStateCommon::new(entity, diagram),
            style_state_name: get_style_state(
                Some(SName::Name),
                entity.stereotype.as_ref(),
                &diagram.skin().current_style_builder(),
            ),
        }
    }
}

impl TextBlock for EntityImageStateEmptyDescription {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let margin = f64::from(MARGIN * 2);
        self.common
            .name
            .calculate_dimension(string_bounder)
            .delta(margin, margin)
            .at_least(MIN_WIDTH, MIN_HEIGHT)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let common = &self.common;
        if let Some(url) = &common.url {
            ug.start_url(url);
        }
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_header = common.name.calculate_dimension(string_bounder);
        let stroke = common.style_state.stroke_with(&common.colors);
        let inner = common
            .apply_color(ug, &self.style_state_name)
            .with_stroke(stroke);
        inner.draw(&common.get_shape(dim_total));
        let x_desc = (dim_total.width - dim_header.width) / 2.0;
        let y_desc = (dim_total.height - dim_header.height) / 2.0;
        common.name.draw_u(&inner.translated(x_desc, y_desc));
        if common.url.is_some() {
            ug.close_url();
        }
    }
}

impl IEntityImage for EntityImageStateEmptyDescription {}
