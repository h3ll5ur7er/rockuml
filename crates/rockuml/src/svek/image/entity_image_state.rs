//! A state: a rounded box with its name above a line and its description below (PlantUML's
//! `EntityImageState`).

use super::entity_group;
use super::entity_image_state_common::{EntityImageStateCommon, get_style_state};
use crate::abel::Entity;
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::shape::{UEllipse, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, Style, ValueReading};
use crate::svek::rounded_container::RoundedContainer;
use crate::svek::{IEntityImage, MARGIN, MARGIN_LINE, ShapeType};

const MIN_WIDTH: f64 = 50.0;
const MIN_HEIGHT: f64 = 50.0;
const SMALL_RADIUS: f64 = 3.0;
const SMALL_LINE: f64 = 3.0;
const SMALL_MARGIN_X: f64 = 7.0;
const SMALL_MARGIN_Y: f64 = 4.0;

pub(crate) struct EntityImageState {
    common: EntityImageStateCommon,
    group: UGroup,
    fields: SheetBlock2,
    /// `<<O-O>>` draws two linked circles in the corner.
    with_symbol: bool,
    style_name: Style,
    style_description: Style,
}

impl EntityImageState {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let common = EntityImageStateCommon::new(entity, diagram);
        let stereotype = entity.stereotype.as_ref();
        let with_symbol = stereotype.is_some_and(|stereotype| {
            stereotype
                .label_double_comparator()
                .eq_ignore_ascii_case("<<O-O>>")
        });
        let builder = diagram.skin().current_style_builder();
        let style_name = get_style_state(Some(SName::Name), stereotype, &builder);
        let style_description = get_style_state(Some(SName::Description), stereotype, &builder);
        let fields = Display::create(entity.bodier.get_raw_body().iter().cloned()).create0(
            &style_description.font_configuration(),
            description_alignment(&style_description),
            diagram.skin(),
            common.style_state.wrap_width(),
            CreoleMode::Full,
        );
        Self {
            group: entity_group(entity, diagram, "entity", entity.get_location()),
            common,
            fields,
            with_symbol,
            style_name,
            style_description,
        }
    }

    /// Two small circles joined by a line, in the bottom right corner.
    pub(crate) fn draw_symbol(ug: &UGraphic, x_symbol: f64, y_symbol: f64) {
        let x_symbol = x_symbol - (4.0 * SMALL_RADIUS + SMALL_LINE + SMALL_MARGIN_X);
        let y_symbol = y_symbol - (2.0 * SMALL_RADIUS + SMALL_MARGIN_Y);
        let small = UShape::Ellipse(UEllipse::new(2.0 * SMALL_RADIUS, 2.0 * SMALL_RADIUS));
        ug.translated(x_symbol, y_symbol).draw(&small);
        ug.translated(x_symbol + SMALL_LINE + 2.0 * SMALL_RADIUS, y_symbol)
            .draw(&small);
        ug.translated(x_symbol + 2.0 * SMALL_RADIUS, y_symbol + SMALL_LINE)
            .draw(&UShape::Line {
                dx: SMALL_LINE,
                dy: 0.0,
            });
    }
}

fn description_alignment(style: &Style) -> HorizontalAlignment {
    style.horizontal_alignment().unwrap_or_default()
}

impl TextBlock for EntityImageState {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim = self
            .common
            .name
            .calculate_dimension(string_bounder)
            .merge_top_bottom(self.fields.calculate_dimension(string_bounder));
        let height_symbol = if self.with_symbol {
            2.0 * SMALL_RADIUS + SMALL_MARGIN_Y
        } else {
            0.0
        };
        let delta = f64::from(MARGIN * 2 + 2 * MARGIN_LINE) + height_symbol;
        dim.delta(delta, delta).at_least(MIN_WIDTH, MIN_HEIGHT)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let common = &self.common;
        ug.start_group(&self.group);
        if let Some(url) = &common.url {
            ug.start_url(url);
        }
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_name = common.name.calculate_dimension(string_bounder);
        let stroke = common
            .colors
            .get_specific_line_stroke()
            .unwrap_or_else(|| common.style_state.stroke());
        let inner = common
            .apply_color(ug, &common.style_state)
            .with_stroke(stroke);
        let y_line = f64::from(MARGIN) + dim_name.height + f64::from(MARGIN_LINE);
        let north_backcolor = self.style_name.value(PName::BackGroundColor).as_color();
        let south_backcolor = self
            .style_description
            .value(PName::BackGroundColor)
            .as_color();
        if north_backcolor == south_backcolor {
            inner.draw(&common.get_shape(dim_total));
        } else {
            RoundedContainer {
                dim: dim_total,
                name_height: y_line,
                description_height: 0.0,
                border_color: common.get_border_color(),
                north_backcolor,
                center_backcolor: south_backcolor.clone(),
                south_backcolor,
                stroke,
                rounded: common.style_state.value(PName::RoundCorner).as_double(),
            }
            .draw_u(&inner);
        }
        inner.translated(0.0, y_line).draw(&UShape::Line {
            dx: dim_total.width,
            dy: 0.0,
        });
        if self.with_symbol {
            Self::draw_symbol(&inner, dim_total.width, dim_total.height);
        }
        let x_desc = (dim_total.width - dim_name.width) / 2.0;
        common
            .name
            .draw_u(&inner.translated(x_desc, f64::from(MARGIN)));
        description_alignment(&self.style_description).draw(
            &inner,
            &self.fields,
            f64::from(MARGIN),
            y_line + f64::from(MARGIN_LINE),
            dim_total.width,
        );
        if common.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }
}

impl IEntityImage for EntityImageState {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::RoundRectangle
    }

    fn is_hidden(&self) -> bool {
        self.common.base.is_hidden()
    }
}
