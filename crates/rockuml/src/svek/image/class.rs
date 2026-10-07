//! A class, interface, enum and the like: a box with a header (spot, stereotype, name, generic) above its
//! members (PlantUML's `EntityImageClass`, `EntityImageClassHeader` and `HeaderLayout`).

use std::rc::Rc;

use super::entity_group;
use crate::abel::{Entity, EntityPortion, LeafType};
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, Display};
use crate::cucadiagram::BodyContext;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::blocks::{CircledCharacter, TextBlockHorizontal, TextBlockMarged};
use crate::klimt::font::{FontStyle, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D, XRectangle2D};
use crate::klimt::group::UGroup;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::skin::font_param::FontParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage};

pub(crate) struct EntityImageClass {
    image: AbstractEntityImage,
    body: Option<Box<dyn TextBlock>>,
    header: HeaderLayout,
    url: Option<Url>,
    group: UGroup,
    comment: String,
    round_corner: f64,
    minimum_width: f64,
    border_color: HColor,
    header_backcolor: HColor,
    backcolor: HColor,
    stroke: UStroke,
}

impl EntityImageClass {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let skin = diagram.skin();
        let entity_builder = entity_style_builder(entity, skin);
        let style = class_signature(entity, &[])
            .get_merged_style_with(&entity_builder, entity.stereotype.as_ref());
        // The header's text takes the rules in force at the end, its background those of the declaration.
        let header_back_style = class_signature(entity, &[SName::Header])
            .get_merged_style_with(&entity_builder, entity.stereotype.as_ref());
        let builder = skin.current_style_builder();
        let style_header = class_signature(entity, &[SName::Header])
            .get_merged_style_with(&builder, entity.stereotype.as_ref());
        let context = BodyContext {
            skin,
            style_builder: &builder,
            style: &style,
            colors: &entity.colors,
            hidden: diagram.get_hides_visibility_modifier(),
        };
        let body = entity.bodier.get_body(
            &context,
            diagram.show_portion(EntityPortion::Method, entity.id()),
            diagram.show_portion(EntityPortion::Field, entity.id()),
        );
        let colors = &entity.colors;
        let backcolor = colors.get(ColorType::Back).cloned();
        let header_backcolor = colors
            .get(ColorType::Header)
            .cloned()
            .or_else(|| backcolor.clone())
            .unwrap_or_else(|| header_back_style.value(PName::BackGroundColor).as_color());
        Self {
            image: AbstractEntityImage::new(entity, diagram),
            body,
            header: header(entity, diagram, &style_header),
            url: entity.url.clone(),
            group: entity_group(entity, diagram, "entity", entity.get_location()),
            comment: format!("class {}", entity.get_name(diagram)),
            round_corner: style.value(PName::RoundCorner).as_double(),
            minimum_width: style.value(PName::MinimumWidth).as_double(),
            border_color: colors
                .get(ColorType::Line)
                .cloned()
                .unwrap_or_else(|| style.value(PName::LineColor).as_color()),
            header_backcolor,
            backcolor: backcolor.unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            stroke: style.stroke_with(colors),
        }
    }

    fn draw_internal(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let dim_header = self.header.get_dimension(string_bounder);
        let (width, height) = (dim_total.width, dim_total.height);
        let rect = UShape::Rectangle(URectangle::new(width, height).rounded(self.round_corner));
        let ug = ug
            .with_color(self.border_color.clone())
            .with_backcolor(self.backcolor.clone());
        let ug_stroke = ug.with_stroke(self.stroke);
        let mut ug_header = ug.clone();
        if self.backcolor == self.header_backcolor {
            ug_stroke.draw(&rect);
        } else if self.round_corner == 0.0 {
            ug_stroke.draw(&rect);
            ug_header = ug_header.with_backcolor(self.header_backcolor.clone());
            ug_header
                .with_stroke(self.stroke)
                .draw(&UShape::Rectangle(URectangle::new(
                    width,
                    dim_header.height,
                )));
        } else {
            ug_stroke.draw(&rect);
            let rect2 = URectangle::new(width, dim_header.height).rounded(self.round_corner);
            let rect3 = URectangle::new(width, self.round_corner / 2.0);
            ug_header = ug_header
                .with_backcolor(self.header_backcolor.clone())
                .with_color(self.header_backcolor.clone());
            let header_stroke = ug_header.with_stroke(self.stroke);
            header_stroke.draw(&UShape::Rectangle(rect2));
            header_stroke
                .translated(0.0, dim_header.height - rect3.height)
                .draw(&UShape::Rectangle(rect3));
            ug_stroke.with_backcolor(HColor::NONE).draw(&rect);
        }
        self.header.draw_u(&ug_header, width, dim_header.height);
        if let Some(body) = &self.body {
            let ug2 = ug.with_stencil_stroke(Rc::new(RectangleStencil { width }), self.stroke);
            body.draw_u(&ug2.translated(0.0, dim_header.height));
        }
    }
}

impl TextBlock for EntityImageClass {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_header = self.header.get_dimension(string_bounder);
        let dim_body = self
            .body
            .as_ref()
            .map_or_else(XDimension2D::default, |body| {
                body.calculate_dimension(string_bounder)
            });
        let width = dim_body.width.max(dim_header.width).max(self.minimum_width);
        XDimension2D::new(width, dim_body.height + dim_header.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::Comment(self.comment.clone()));
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        ug.start_group(&self.group);
        self.draw_internal(ug);
        ug.close_group();
        if self.url.is_some() {
            ug.close_url();
        }
    }

    fn backcolor(&self) -> Option<HColor> {
        Some(self.image.get_backcolor())
    }

    fn get_inner_position(
        &self,
        member: &str,
        string_bounder: &dyn StringBounder,
    ) -> Option<XRectangle2D> {
        let header_height = self.header.get_dimension(string_bounder).height;
        self.body
            .as_ref()?
            .get_inner_position(member, string_bounder)
            .map(|position| position.translated(0.0, header_height))
    }
}

impl IEntityImage for EntityImageClass {}

/// `root element classDiagram class`, then `more`, with the entity's `<<<style>>>` names.
fn class_signature(entity: &Entity, more: &[SName]) -> StyleSignature {
    let mut names = vec![
        SName::Root,
        SName::Element,
        SName::ClassDiagram,
        SName::Class,
    ];
    names.extend_from_slice(more);
    StyleSignature::of(&names).with_stereostyles(&entity.stereostyles)
}

/// The rules in force when the entity was declared.
fn entity_style_builder(entity: &Entity, skin: &SkinParam) -> Rc<crate::style::StyleBuilder> {
    entity
        .style_builder()
        .cloned()
        .unwrap_or_else(|| skin.current_style_builder())
}

/// `EntityImageClassHeader`.
fn header(entity: &Entity, diagram: &CucaDiagram, style_header: &Style) -> HeaderLayout {
    let skin = diagram.skin();
    let leaf_type = entity
        .get_leaf_type()
        .expect("only leaves are drawn as classes");
    let stereotype = entity.stereotype.as_ref();
    let mut font = style_header.font_configuration_with(&entity.colors);
    if matches!(leaf_type, LeafType::AbstractClass | LeafType::Interface) {
        font = font.with_style(FontStyle::Italic);
    }
    let old_fashion = skin.display_generic_with_old_fashion();
    let generic = entity.generic.as_deref().filter(|_| !old_fashion);
    let mut display = entity.display.clone();
    if old_fashion && let Some(generic) = &entity.generic {
        display = display.add_generic(generic);
    }
    let display = display.create0(
        &font,
        HorizontalAlignment::Center,
        skin,
        style_header.wrap_width(),
        CreoleMode::FullButUnderscore,
    );
    let mut name: Box<dyn TextBlock> = Box::new(display);
    if let Some(modifier) = entity.visibility_modifier {
        let back = modifier.get_background().map(|color| color.get(skin));
        let icon = modifier.get_u_block(
            skin.class_attribute_icon_size(),
            modifier.get_foreground().get(skin),
            back,
            false,
        );
        name = Box::new(TextBlockHorizontal {
            left: Box::new(TextBlockMarged::new(
                icon,
                ClockwiseTopRightBottomLeft::top_right_bottom_left(4.0, 0.0, 0.0, 0.0),
            )),
            right: name,
        });
    }
    let name = Box::new(TextBlockMarged::new(
        name,
        ClockwiseTopRightBottomLeft::margin1_margin2(0.0, 3.0),
    ));
    let stereotype_font = skin.get_font_configuration(FontParam::ClassStereotype, stereotype);
    let stereo = diagram
        .get_visible_stereotype_labels(entity.id())
        .filter(|labels| !labels.is_empty())
        .map(|labels| -> Box<dyn TextBlock> {
            let block = Display::create(labels).create0(
                &stereotype_font,
                HorizontalAlignment::Center,
                skin,
                0.0,
                CreoleMode::Full,
            );
            Box::new(TextBlockMarged::new(
                block,
                ClockwiseTopRightBottomLeft::margin1_margin2(0.0, 1.0),
            ))
        });
    let generic = generic.map(|generic| -> Box<dyn TextBlock> {
        let style_generic = class_signature(entity, &[SName::Generic])
            .get_merged_style_with(&skin.current_style_builder(), stereotype);
        let block = Display::with_newlines(generic).create0(
            &stereotype_font,
            HorizontalAlignment::Center,
            skin,
            0.0,
            CreoleMode::Full,
        );
        let block = TextBlockGeneric {
            block: Box::new(TextBlockMarged::new(
                block,
                ClockwiseTopRightBottomLeft::same(1.0),
            )),
            background: style_generic.value(PName::BackGroundColor).as_color(),
            border: style_generic.value(PName::LineColor).as_color(),
        };
        Box::new(TextBlockMarged::new(
            block,
            ClockwiseTopRightBottomLeft::same(1.0),
        ))
    });
    let circled_character = diagram
        .show_portion(EntityPortion::CircledCharacter, entity.id())
        .then(|| -> Box<dyn TextBlock> {
            Box::new(TextBlockMarged::new(
                circled_character(entity, leaf_type, skin),
                ClockwiseTopRightBottomLeft::top_right_bottom_left(5.0, 0.0, 5.0, 4.0),
            ))
        });
    HeaderLayout {
        circled_character: circled_character.unwrap_or_else(empty),
        stereo: stereo.unwrap_or_else(empty),
        name,
        generic: generic.unwrap_or_else(empty),
    }
}

fn empty() -> Box<dyn TextBlock> {
    Box::new(TextBlockEmpty::default())
}

/// The spot: a letter for the kind of class, or the stereotype's own.
fn circled_character(entity: &Entity, leaf_type: LeafType, skin: &SkinParam) -> CircledCharacter {
    let font = skin.get_font(FontParam::CircledCharacter, None);
    let style = StyleSignature::of(&[
        SName::Root,
        SName::Element,
        SName::Spot,
        spot_name(leaf_type),
    ])
    .get_merged_style(&skin.current_style_builder());
    let spot_border = style.value(PName::LineColor).as_color();
    let spot_back = style.value(PName::BackGroundColor).as_color();
    let font_color = style.value(PName::FontColor).as_color();
    let radius = f64::from(skin.get_circled_character_radius());
    let stereotype = entity.stereotype.as_ref();
    let (character, back) = match stereotype.and_then(|stereotype| stereotype.spot()) {
        Some(spot) => (spot.character, spot.color.clone().unwrap_or(spot_back)),
        None => (
            stereotype
                .and_then(|stereotype| skin.get_circled_character(stereotype))
                .unwrap_or_else(|| circled_char(leaf_type)),
            spot_back,
        ),
    };
    CircledCharacter::new(character, radius, font, Some(back), font_color).with_border(spot_border)
}

fn spot_name(leaf_type: LeafType) -> SName {
    match leaf_type {
        LeafType::Annotation => SName::SpotAnnotation,
        LeafType::AbstractClass => SName::SpotAbstractClass,
        LeafType::Class => SName::SpotClass,
        LeafType::Interface => SName::SpotInterface,
        LeafType::Enum => SName::SpotEnum,
        LeafType::Entity => SName::SpotEntity,
        LeafType::Protocol => SName::SpotProtocol,
        LeafType::Struct => SName::SpotStruct,
        LeafType::Exception => SName::SpotException,
        LeafType::Metaclass => SName::SpotMetaClass,
        LeafType::Stereotype => SName::SpotStereotype,
        LeafType::Dataclass => SName::SpotDataClass,
        LeafType::Record => SName::SpotRecord,
        _ => unreachable!("only class-like leaves have spots"),
    }
}

fn circled_char(leaf_type: LeafType) -> char {
    match leaf_type {
        LeafType::Annotation => '@',
        LeafType::AbstractClass => 'A',
        LeafType::Class => 'C',
        LeafType::Interface => 'I',
        LeafType::Enum | LeafType::Entity => 'E',
        LeafType::Protocol => 'P',
        LeafType::Struct | LeafType::Stereotype => 'S',
        LeafType::Exception => 'X',
        LeafType::Metaclass => 'M',
        LeafType::Dataclass => 'D',
        LeafType::Record => 'R',
        _ => '?',
    }
}

/// A generic in its dashed box (`TextBlockGeneric`).
struct TextBlockGeneric {
    block: Box<dyn TextBlock>,
    background: HColor,
    border: HColor,
}

impl TextBlock for TextBlockGeneric {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.block.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug
            .with_backcolor(self.background.clone())
            .with_color(self.border.clone());
        let dimension = self.calculate_dimension(ug.string_bounder());
        ug.with_stroke(UStroke {
            dash_visible: 2.0,
            dash_space: 2.0,
            thickness: 1.0,
        })
        .draw(&UShape::Rectangle(URectangle::new(
            dimension.width,
            dimension.height,
        )));
        self.block.draw_u(&ug);
    }
}

/// Where the spot, the stereotype, the name and the generic of a header go (`HeaderLayout`).
struct HeaderLayout {
    circled_character: Box<dyn TextBlock>,
    stereo: Box<dyn TextBlock>,
    name: Box<dyn TextBlock>,
    generic: Box<dyn TextBlock>,
}

impl HeaderLayout {
    fn get_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let name = self.name.calculate_dimension(string_bounder);
        let generic = self.generic.calculate_dimension(string_bounder);
        let stereo = self.stereo.calculate_dimension(string_bounder);
        let circle = self.circled_character.calculate_dimension(string_bounder);
        let width = circle.width + stereo.width.max(name.width) + generic.width;
        let height = circle
            .height
            .max(stereo.height + name.height + 10.0)
            .max(generic.height);
        XDimension2D::new(width, height)
    }

    fn draw_u(&self, ug: &UGraphic, width: f64, height: f64) {
        let string_bounder = ug.string_bounder();
        let name = self.name.calculate_dimension(string_bounder);
        let generic = self.generic.calculate_dimension(string_bounder);
        let stereo = self.stereo.calculate_dimension(string_bounder);
        let circle = self.circled_character.calculate_dimension(string_bounder);
        let width_stereo_and_name = stereo.width.max(name.width);
        let supp_width = (width - circle.width - width_stereo_and_name - generic.width).max(0.0);
        let h2 = (circle.width / 4.0).min(supp_width * 0.1);
        let h1 = (supp_width - h2) / 2.0;
        self.circled_character
            .draw_u(&ug.translated(h1, (height - circle.height) / 2.0));
        let diff_height = height - stereo.height - name.height;
        let x_stereo = circle.width + (width_stereo_and_name - stereo.width) / 2.0 + h1 + h2;
        self.stereo
            .draw_u(&ug.translated(x_stereo, diff_height / 2.0));
        let x_name = circle.width + (width_stereo_and_name - name.width) / 2.0 + h1 + h2;
        self.name
            .draw_u(&ug.translated(x_name, diff_height / 2.0 + stereo.height));
        if generic.width > 0.0 {
            let delta = 4.0;
            self.generic
                .draw_u(&ug.translated(width - generic.width + delta, -delta));
        }
    }
}
