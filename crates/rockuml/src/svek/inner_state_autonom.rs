//! A composite state laid out on its own: a rounded box with its name and description on top and the layout
//! of its inside below (PlantUML's `InnerStateAutonom`).

use super::image::{EntityImageState, entity_group, get_state_description, get_style_state};
use super::rounded_container::RoundedContainer;
use super::{IEntityImage, MARGIN, MARGIN_LINE, ShapeType};
use crate::abel::Entity;
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::UGroup;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, ValueReading};

pub(crate) struct InnerStateAutonom {
    im: Box<dyn IEntityImage>,
    group: UGroup,
    name: SheetBlock2,
    attribute: Box<dyn TextBlock>,
    border_color: HColor,
    url: Option<Url>,
    /// `<<O-O>>` draws two linked circles in the corner.
    with_symbol: bool,
    stroke: UStroke,
    rounded: f64,
    description_alignment: HorizontalAlignment,
    north_backcolor: HColor,
    center_backcolor: HColor,
    south_backcolor: HColor,
}

impl InnerStateAutonom {
    pub(crate) fn new(im: Box<dyn IEntityImage>, group: &Entity, diagram: &CucaDiagram) -> Self {
        let builder = diagram.skin().current_style_builder();
        let stereotype = group.stereotype.as_ref();
        let style_part = |part| get_style_state(part, stereotype, &builder);
        let style_name = style_part(Some(SName::Name));
        let style = style_part(None);
        let style_description = style_part(Some(SName::Description));
        let name = group.display.create0(
            style_name.font_configuration(),
            style_name.horizontal_alignment().unwrap_or_default(),
            diagram.skin(),
            0.0,
            CreoleMode::Full,
        );
        let back = |part| {
            style_part(Some(part))
                .value(PName::BackGroundColor)
                .as_color()
        };
        let (north_backcolor, center_backcolor, south_backcolor) =
            group.colors.get(ColorType::Back).map_or_else(
                || {
                    (
                        back(SName::Name),
                        back(SName::Description),
                        back(SName::Body),
                    )
                },
                |own| (own.clone(), own.clone(), own.clone()),
            );
        Self {
            im,
            group: entity_group(group, diagram, "entity", group.get_location()),
            name,
            attribute: get_state_description(group, diagram),
            border_color: group
                .colors
                .get(ColorType::Line)
                .cloned()
                .unwrap_or_else(|| style.value(PName::LineColor).as_color()),
            url: group.url.clone(),
            with_symbol: group.stereotype.as_ref().is_some_and(|stereotype| {
                stereotype
                    .label_double_comparator()
                    .eq_ignore_ascii_case("<<O-O>>")
            }),
            stroke: group
                .colors
                .get_specific_line_stroke()
                .unwrap_or_else(|| style.stroke()),
            rounded: style.value(PName::RoundCorner).as_double(),
            description_alignment: style_description.horizontal_alignment().unwrap_or_default(),
            north_backcolor,
            center_backcolor,
            south_backcolor,
        }
    }

    /// The description takes some room below it only if there is one.
    fn margin_for_fields(attribute: XDimension2D) -> f64 {
        if attribute.height > 0.0 {
            f64::from(MARGIN)
        } else {
            0.0
        }
    }

    /// Where the inside's layout starts, below the name and description.
    fn get_space_y_for_url(&self, string_bounder: &dyn StringBounder) -> f64 {
        let text = self.name.calculate_dimension(string_bounder);
        let attr = self.attribute.calculate_dimension(string_bounder);
        let titre_height = f64::from(MARGIN) + text.height + f64::from(MARGIN_LINE);
        let supp_y = titre_height + Self::margin_for_fields(attr) + attr.height;
        supp_y + f64::from(MARGIN_LINE)
    }
}

impl TextBlock for InnerStateAutonom {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let img = self.im.calculate_dimension(string_bounder);
        let text = self.name.calculate_dimension(string_bounder);
        let attr = self.attribute.calculate_dimension(string_bounder);
        let dim = XDimension2D::new(
            text.width.max(attr.width).max(img.width),
            text.height + attr.height + img.height,
        );
        let delta = f64::from(MARGIN * 2 + 2 * MARGIN_LINE) + Self::margin_for_fields(attr);
        dim.delta(delta, delta)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let text = self.name.calculate_dimension(string_bounder);
        let attr = self.attribute.calculate_dimension(string_bounder);
        let total = self.calculate_dimension(string_bounder);
        let container = RoundedContainer {
            dim: total,
            name_height: f64::from(MARGIN) + text.height + f64::from(MARGIN_LINE),
            description_height: attr.height + Self::margin_for_fields(attr),
            border_color: self.border_color.clone(),
            north_backcolor: self.north_backcolor.clone(),
            center_backcolor: self.center_backcolor.clone(),
            south_backcolor: self.south_backcolor.clone(),
            stroke: self.stroke,
            rounded: self.rounded,
        };
        ug.start_group(&self.group);
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        container.draw_u(ug);
        self.name
            .draw_u(&ug.translated((total.width - text.width) / 2.0, f64::from(MARGIN)));
        self.description_alignment.draw(
            ug,
            self.attribute.as_ref(),
            f64::from(MARGIN),
            f64::from(MARGIN) + text.height + f64::from(MARGIN),
            total.width,
        );
        self.im
            .draw_u(&ug.translated(f64::from(MARGIN), self.get_space_y_for_url(string_bounder)));
        if self.with_symbol {
            EntityImageState::draw_symbol(
                &ug.with_color(self.border_color.clone()),
                total.width,
                total.height,
            );
        }
        if self.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }
}

impl IEntityImage for InnerStateAutonom {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::RoundRectangle
    }

    fn is_hidden(&self) -> bool {
        self.im.is_hidden()
    }
}
