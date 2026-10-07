//! An object: a box with its name, underlined in strict UML, above its fields (PlantUML's
//! `EntityImageObject`).

use std::rc::Rc;

use super::entity_group;
use crate::abel::{Entity, EntityPortion};
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, Display};
use crate::cucadiagram::BodyContext;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::blocks::{TextBlockLineBefore, TextBlockMarged};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{XDimension2D, XRectangle2D};
use crate::klimt::group::UGroup;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;
use crate::skin::font_param::FontParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage};

/// How tall an object without fields is.
const MARGIN_EMPTY_FIELDS_OR_METHOD: f64 = 13.0;
/// The room each side of the title.
const X_MARGIN_CIRCLE: f64 = 5.0;

pub(crate) struct EntityImageObject {
    image: AbstractEntityImage,
    name: Box<dyn TextBlock>,
    stereo: Option<Box<dyn TextBlock>>,
    fields: Box<dyn TextBlock>,
    show_fields: bool,
    url: Option<Url>,
    group: UGroup,
    round_corner: f64,
    minimum_width: f64,
    border_color: HColor,
    header_backcolor: HColor,
    backcolor: HColor,
    stroke: UStroke,
}

fn object_signature(more: &[SName]) -> StyleSignature {
    let mut names = vec![
        SName::Root,
        SName::Element,
        SName::ObjectDiagram,
        SName::Object,
    ];
    names.extend_from_slice(more);
    StyleSignature::of(&names)
}

impl EntityImageObject {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let skin = diagram.skin();
        let stereotype = entity.stereotype.as_ref();
        let builder = skin.current_style_builder();
        let style = object_signature(&[]).get_merged_style_with(&builder, stereotype);
        let style_header =
            object_signature(&[SName::Header]).get_merged_style_with(&builder, stereotype);
        let mut display = entity.display.clone();
        if skin.strict_uml_style() {
            display = display.underlined();
        }
        let name = display.create0(
            &style_header.font_configuration(),
            HorizontalAlignment::Center,
            skin,
            0.0,
            CreoleMode::Full,
        );
        let name = Box::new(TextBlockMarged::new(name, style.padding()));
        let stereo = stereotype
            .filter(|stereotype| !stereotype.label_double_comparator().is_empty())
            .filter(|_| diagram.show_portion(EntityPortion::Stereotype, entity.id()))
            .map(|stereotype| -> Box<dyn TextBlock> {
                Box::new(Display::create(stereotype.labels()).create0(
                    &skin.get_font_configuration(FontParam::ObjectStereotype, Some(stereotype)),
                    HorizontalAlignment::Center,
                    skin,
                    0.0,
                    CreoleMode::Full,
                ))
            });
        let show_fields = diagram.show_portion(EntityPortion::Field, entity.id());
        let hidden = diagram.get_hides_visibility_modifier();
        let line_thickness = style.value(PName::LineThickness).as_double();
        let fields: Box<dyn TextBlock> =
            if show_fields && entity.bodier.get_fields_to_display(hidden).is_empty() {
                Box::new(TextBlockLineBefore {
                    block: Box::new(TextBlockEmpty {
                        dimension: XDimension2D::new(10.0, 16.0),
                    }),
                    style: '\0',
                    title: None,
                    thickness: line_thickness,
                })
            } else {
                let context = BodyContext {
                    skin,
                    style_builder: &builder,
                    style: &style,
                    colors: &entity.colors,
                    hidden,
                };
                entity
                    .bodier
                    .get_body(&context, false, show_fields)
                    .expect("objects always have a body")
            };
        let colors = &entity.colors;
        let backcolor = colors.get(ColorType::Back).cloned();
        let header_backcolor = colors
            .get(ColorType::Header)
            .cloned()
            .or_else(|| backcolor.clone())
            .unwrap_or_else(|| style_header.value(PName::BackGroundColor).as_color());
        Self {
            image: AbstractEntityImage::new(entity, diagram),
            name,
            stereo,
            fields,
            show_fields,
            url: entity.url.clone(),
            group: entity_group(entity, diagram, "entity", entity.get_location()),
            round_corner: style.value(PName::RoundCorner).as_double(),
            minimum_width: style.value(PName::MinimumWidth).as_double(),
            border_color: style.value(PName::LineColor).as_color(),
            header_backcolor,
            backcolor: backcolor.unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            stroke: style.stroke(),
        }
    }

    fn get_title_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        name_and_stereotype_dimension(&*self.name, self.stereo.as_deref(), string_bounder)
    }
}

/// The name under the stereotype.
pub(super) fn name_and_stereotype_dimension(
    name: &dyn TextBlock,
    stereo: Option<&dyn TextBlock>,
    string_bounder: &dyn StringBounder,
) -> XDimension2D {
    let name = name.calculate_dimension(string_bounder);
    let stereo = stereo.map_or_else(XDimension2D::default, |stereo| {
        stereo.calculate_dimension(string_bounder)
    });
    XDimension2D::new(name.width.max(stereo.width), name.height + stereo.height)
}

/// The stereotype and the name, each centred, sharing the room left over above, between and below them
/// (`PlacementStrategyY1Y2`), or aligned by `alignment`.
pub(super) fn draw_title(
    ug: &UGraphic,
    blocks: &[&dyn TextBlock],
    width: f64,
    height: f64,
    alignment: HorizontalAlignment,
) {
    let string_bounder = ug.string_bounder();
    let dimensions: Vec<XDimension2D> = blocks
        .iter()
        .map(|block| block.calculate_dimension(string_bounder))
        .collect();
    let used: f64 = dimensions.iter().map(|dimension| dimension.height).sum();
    let space = (height - used) / (blocks.len() as f64 + 1.0);
    let mut y = space;
    for (block, dimension) in blocks.iter().zip(&dimensions) {
        block.draw_u(&ug.translated(alignment.offset(width, dimension.width), y));
        y += dimension.height + space;
    }
}

impl TextBlock for EntityImageObject {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let title = self.get_title_dimension(string_bounder);
        let fields = self.fields.calculate_dimension(string_bounder);
        let width = fields
            .width
            .max(title.width + 2.0 * X_MARGIN_CIRCLE)
            .max(self.minimum_width);
        let fields_height = if fields.height == 0.0 && self.show_fields {
            MARGIN_EMPTY_FIELDS_OR_METHOD
        } else {
            fields.height
        };
        XDimension2D::new(width, fields_height + title.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let total = self.calculate_dimension(string_bounder);
        let title = self.get_title_dimension(string_bounder);
        let ug = ug
            .with_color(self.border_color.clone())
            .with_backcolor(self.backcolor.clone());
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        ug.start_group(&self.group);
        ug.with_stroke(self.stroke).draw(&UShape::Rectangle(
            URectangle::new(total.width, total.height).rounded(self.round_corner),
        ));
        if self.backcolor != self.header_backcolor {
            ug.with_backcolor(self.header_backcolor.clone())
                .with_stroke(self.stroke)
                .draw(&URectangle::new(total.width, title.height).half_rounded(self.round_corner));
        }
        let mut header: Vec<&dyn TextBlock> = Vec::new();
        if let Some(stereo) = &self.stereo {
            header.push(stereo.as_ref());
        }
        header.push(self.name.as_ref());
        draw_title(
            &ug,
            &header,
            total.width,
            title.height,
            HorizontalAlignment::Center,
        );
        let ug2 = ug.with_stencil_stroke(
            Rc::new(RectangleStencil { width: total.width }),
            self.stroke,
        );
        self.fields.draw_u(&ug2.translated(0.0, title.height));
        if self.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.image.get_backcolor())
    }

    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        let title_height = self.get_title_dimension(string_bounder).height;
        self.fields
            .get_inner_position(member, string_bounder)
            .map(|position| position.translated(0.0, title_height))
    }
}

impl IEntityImage for EntityImageObject {}
