//! A description element drawn as its symbol around its name and stereotype: actors, use cases,
//! components, nodes, interfaces... (PlantUML's `EntityImageDescription`).

use std::rc::Rc;

use crate::abel::{Entity, LeafType};
use crate::color::ColorType;
use crate::creole::Display;
use crate::decoration::symbol::{Block, USymbols};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::blocks::{TextBlockMarged, TextBlockVertical};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, UTranslate, XDimension2D, XPoint2D};
use crate::klimt::group::UGroup;
use crate::klimt::shape::UShape;
use crate::klimt::sprite::SpriteContainer;
use crate::klimt::stencil::RectangleStencil;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::body::enhanced_text;
use crate::skin::component::{TextBlockEmpty, creole_text};
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};
use crate::svek::{AbstractEntityImage, IEntityImage, ShapeType};

use super::entity_group;

pub(crate) struct EntityImageDescription {
    base: AbstractEntityImage,
    shape_type: ShapeType,
    url: Option<Url>,
    as_small: Box<dyn TextBlock>,
    desc: Block,
    stereo: Block,
    /// Interfaces draw their circle alone, their name below and their stereotype above it.
    hide_text: bool,
    comment: String,
    group: UGroup,
}

impl EntityImageDescription {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let skin = diagram.skin();
        let symbol = entity
            .get_usymbol()
            .unwrap_or_else(|| skin.component_style().to_u_symbol());
        let shape_type = match symbol {
            USymbols::FOLDER | USymbols::PACKAGE => ShapeType::Folder,
            USymbols::HEXAGON => ShapeType::Hexagon,
            USymbols::USECASE | USymbols::USECASE_BUSINESS => ShapeType::Oval,
            _ => ShapeType::Rectangle,
        };
        let hide_text = symbol == USymbols::INTERFACE;
        let colors = &entity.colors;
        let style_name = diagram.get_style_name();
        let snames = symbol.get_s_names();
        let element = |extra: &[SName]| {
            let mut names = vec![SName::Root, SName::Element, style_name];
            names.extend(&snames);
            names.extend(extra);
            StyleSignature::of(&names)
        };
        let signature_title = if symbol == USymbols::ACTOR_STICKMAN_BUSINESS {
            StyleSignature::of(&[
                SName::Root,
                SName::Element,
                style_name,
                SName::Actor,
                SName::Business,
                SName::Title,
            ])
        } else {
            element(&[SName::Title])
        };
        let builder = entity
            .style_builder()
            .expect("only the root has no style builder");
        let stereotype = entity.stereotype.as_ref();
        let style_title = signature_title
            .get_merged_style_with(builder, stereotype)
            .eventually_override_colors(colors);
        let style_stereo = element(&[SName::Stereotype])
            .get_merged_style_for_stereotype_itself(builder, stereotype);
        let style = element(&[])
            .get_merged_style_with(builder, stereotype)
            .eventually_override_colors(colors);

        let forecolor = style_title.value(PName::LineColor).as_color();
        let backcolor = colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style_title.value(PName::BackGroundColor).as_color());
        let fashion = Fashion::new(backcolor, forecolor)
            .with_stroke(style_title.stroke_with(colors))
            .with_corner(
                style_title.value(PName::RoundCorner).as_double(),
                style_title.value(PName::DiagonalCorner).as_double(),
            );
        let fc_title = style_title.font_configuration();
        let fc_stereo = style_stereo.font_configuration();
        let default_align = style_title.horizontal_alignment().unwrap_or_default();

        let name = entity.get_name(diagram);
        let code_display = Display::with_newlines(name);
        let display = &entity.display;
        let is_code = display.lines() == code_display.lines();
        let desc: Block = if (is_code && snames[0] == SName::Package) || display.is_white() {
            Rc::new(TextBlockEmpty {
                dimension: XDimension2D::new(style.value(PName::MinimumWidth).as_double(), 0.0),
            })
        } else if is_code {
            Rc::from(enhanced_text(
                display,
                fc_title.clone(),
                default_align,
                &style_title,
                skin,
            ))
        } else {
            Rc::from(enhanced_text(
                display,
                style.font_configuration(),
                default_align,
                &style,
                skin,
            ))
        };

        let stereo: Block = match (stereotype, stereotype.and_then(|s| s.get_sprite(skin))) {
            (_, Some(sprite)) => Rc::from(sprite),
            (Some(_), None) => match diagram.get_visible_stereotype_labels(entity.id()) {
                Some(labels) if !labels.is_empty() => Rc::new(TextBlockMarged::new(
                    creole_text(&labels, fc_stereo, HorizontalAlignment::Center, 0.0, skin),
                    ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 1.0, 0.0, 1.0),
                )),
                _ => Rc::new(TextBlockEmpty::default()),
            },
            (None, None) => Rc::new(TextBlockEmpty::default()),
        };

        let name_block: Block = Rc::new(name_block(
            &code_display,
            entity,
            skin.get_default_text_alignment(HorizontalAlignment::Center),
            style_title.font_configuration_with(colors),
            &style_title,
            skin,
        ));
        let stereotype_alignment = style_stereo.horizontal_alignment().unwrap_or_default();
        let as_small = if hide_text {
            let empty = || -> Block { Rc::new(TextBlockEmpty::default()) };
            symbol.as_small(empty(), empty(), empty(), fashion, stereotype_alignment)
        } else {
            symbol.as_small(
                name_block,
                desc.clone(),
                stereo.clone(),
                fashion,
                stereotype_alignment,
            )
        };
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            shape_type,
            url: entity.url.clone(),
            as_small,
            desc,
            stereo,
            hide_text,
            comment: format!("entity {name}"),
            group: entity_group(entity, diagram, "entity", entity.get_location()),
        }
    }
}

impl TextBlock for EntityImageDescription {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.as_small.calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        ug.draw(&UShape::Comment(self.comment.clone()));
        ug.start_group(&self.group);
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        // A hexagon's outline is the polygon Graphviz gives its node; Smetana gives none.
        self.as_small.draw_u(ug);
        if self.hide_text {
            const SPACE: f64 = 8.0;
            let string_bounder = ug.string_bounder();
            let dim_small = self.as_small.calculate_dimension(string_bounder);
            let dim_desc = self.desc.calculate_dimension(string_bounder);
            let posx1 = (dim_small.width - dim_desc.width) / 2.0;
            let ug_desc = ug
                .translated(posx1, SPACE + dim_small.height)
                .with_stencil(Rc::new(RectangleStencil {
                    width: dim_desc.width,
                }));
            self.desc.draw_u(&ug_desc);
            let dim_stereo = self.stereo.calculate_dimension(string_bounder);
            let posx2 = (dim_small.width - dim_stereo.width) / 2.0;
            self.stereo
                .draw_u(&ug.translated(posx2, -SPACE - dim_stereo.height));
        }
        if self.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }

    fn magnetic_border_force_at(
        &self,
        string_bounder: &dyn StringBounder,
        position: XPoint2D,
    ) -> UTranslate {
        if self.shape_type == ShapeType::Folder {
            self.as_small
                .magnetic_border_force_at(string_bounder, position)
        } else {
            UTranslate::default()
        }
    }
}

impl IEntityImage for EntityImageDescription {
    fn get_shape_type(&self) -> ShapeType {
        self.shape_type
    }

    fn is_hidden(&self) -> bool {
        self.base.is_hidden()
    }
}

/// The entity's name as a class body draws it (`BodyEnhanced1` on a display with no members): each line
/// centred on the widest, with room on both sides. Only package symbols show it.
fn name_block(
    display: &Display,
    entity: &Entity,
    alignment: HorizontalAlignment,
    font: FontConfiguration,
    style: &Style,
    sprites: &dyn SpriteContainer,
) -> impl TextBlock + 'static {
    let mut lines: Vec<String> = display.lines().to_vec();
    let in_ellipse = matches!(
        entity.get_leaf_type(),
        Some(LeafType::Usecase | LeafType::UsecaseBusiness)
    );
    if in_ellipse && lines.is_empty() {
        lines.push(String::new());
    }
    let blocks = lines
        .iter()
        .map(|line| {
            creole_text(
                &Display::with_newlines(line).lines().to_vec(),
                font.clone(),
                alignment,
                style.wrap_width(),
                sprites,
            )
        })
        .collect();
    TextBlockMarged::new(
        TextBlockVertical::new(blocks, HorizontalAlignment::Center),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 6.0, 0.0, 6.0),
    )
}
