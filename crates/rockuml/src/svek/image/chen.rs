//! The images of Chen diagrams: entities, relationships, attributes and the circles of subclass groups
//! (PlantUML's `EntityImageChenEntity`, `EntityImageChenRelationship`, `EntityImageChenAttribute` and
//! `EntityImageChenCircle`).

use super::super::{AbstractEntityImage, IEntityImage, MARGIN, MARGIN_LINE, ShapeType};
use crate::abel::Entity;
use crate::color::ColorType;
use crate::creole::{CreoleMode, SheetBlock2};
use crate::decoration::symbol::TextBlockInEllipse;
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::font::{FontStyle, StringBounder};
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// What the four Chen images share: the entity, its title and its style.
struct ChenImage {
    base: AbstractEntityImage,
    title: SheetBlock2,
    style: Style,
    /// The quark's qualified name, which PlantUML names the image's group after.
    name: String,
    url: Option<crate::klimt::url::Url>,
    colors: crate::color::Colors,
}

impl ChenImage {
    /// `element` is the style name of the image's elements; `underline` underlines the title, as keys are.
    fn new(entity: &Entity, diagram: &CucaDiagram, element: SName, underline: bool) -> Self {
        let builder = diagram.skin().current_style_builder();
        let signature =
            StyleSignature::of(&[SName::Root, SName::Element, SName::ChenEerDiagram, element]);
        let stereotype = entity.stereotype.as_ref();
        let style = signature.get_merged_style_with(&builder, stereotype);
        let style_title = signature
            .with_name(SName::Title)
            .get_merged_style_with(&builder, stereotype);
        let mut font = style_title.font_configuration_with(&entity.colors);
        if underline {
            font = font.with_style(FontStyle::Underline);
        }
        let title = entity.display.create0(
            &font,
            HorizontalAlignment::Center,
            diagram.skin(),
            style.wrap_width(),
            CreoleMode::Full,
        );
        Self {
            base: AbstractEntityImage::new(entity, diagram),
            title,
            style,
            name: diagram
                .quark(entity.get_quark())
                .get_qualified_name()
                .to_owned(),
            url: entity.url.clone(),
            colors: entity.colors.clone(),
        }
    }

    /// The image's own colours, or its style's.
    fn apply_color(&self, ug: &UGraphic) -> UGraphic {
        let border = self
            .colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| self.style.value(PName::LineColor).as_color());
        let backcolor = self
            .colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| self.style.value(PName::BackGroundColor).as_color());
        ug.with_color(border).with_backcolor(backcolor)
    }

    /// Draws the shapes, then the title at (`x_title`, `y_title`), inside the entity's group and link.
    fn draw(
        &self,
        ug: &UGraphic,
        stroke: UStroke,
        shapes: &[(f64, f64, UShape)],
        title_at: (f64, f64),
    ) {
        ug.start_group(&UGroup::singleton(UGroupType::Id, &self.name));
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        let ug_shape = self.apply_color(ug).with_stroke(stroke);
        for (dx, dy, shape) in shapes {
            ug_shape.translated(*dx, *dy).draw(shape);
        }
        self.title
            .draw_u(&ug_shape.translated(title_at.0, title_at.1));
        if self.url.is_some() {
            ug.close_url();
        }
        ug.close_group();
    }
}

fn has_stereotype(entity: &Entity, stereotype: &str) -> bool {
    entity
        .stereotype
        .as_ref()
        .is_some_and(|known| known.to_string().contains(stereotype))
}

/// A Chen entity: a rectangle, doubled for weak entities.
pub(crate) struct EntityImageChenEntity {
    image: ChenImage,
    is_weak: bool,
}

impl EntityImageChenEntity {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self {
            image: ChenImage::new(entity, diagram, SName::ChenEntity, false),
            is_weak: has_stereotype(entity, "<<weak>>"),
        }
    }
}

impl TextBlock for EntityImageChenEntity {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let margin = f64::from(MARGIN * 2 + 2 * MARGIN_LINE);
        self.image
            .title
            .calculate_dimension(string_bounder)
            .delta(margin, margin)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self.calculate_dimension(ug.string_bounder());
        let dim_title = self.image.title.calculate_dimension(ug.string_bounder());
        let shape = |dim: XDimension2D| UShape::Rectangle(URectangle::new(dim.width, dim.height));
        let mut shapes = vec![(0.0, 0.0, shape(dim_total))];
        if self.is_weak {
            shapes.push((3.0, 3.0, shape(dim_total.delta(-6.0, -6.0))));
        }
        let x_title = (dim_total.width - dim_title.width) / 2.0;
        let y_title = f64::from(MARGIN + MARGIN_LINE);
        self.image
            .draw(ug, self.image.style.stroke(), &shapes, (x_title, y_title));
    }

    fn backcolor(&self) -> Option<crate::color::HColor> {
        Some(self.image.base.get_backcolor())
    }
}

impl IEntityImage for EntityImageChenEntity {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        self.image.base.is_hidden()
    }
}

/// A Chen relationship: a diamond twice as wide as high, doubled for identifying relationships.
pub(crate) struct EntityImageChenRelationship {
    image: ChenImage,
    is_identifying: bool,
}

impl EntityImageChenRelationship {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self {
            image: ChenImage::new(entity, diagram, SName::ChenRelationship, false),
            is_identifying: has_stereotype(entity, "<<identifying>>"),
        }
    }
}

/// A diamond touching the middle of each side of the box.
fn diamond(dim: XDimension2D) -> UShape {
    let (width, height) = (dim.width, dim.height);
    UShape::Polygon(vec![
        (0.0, height / 2.0),
        (width / 2.0, 0.0),
        (width, height / 2.0),
        (width / 2.0, height),
    ])
}

impl TextBlock for EntityImageChenRelationship {
    /// The diamond around the title: its diagonal is the title's extent along the direction (1, 2).
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_title = self.image.title.calculate_dimension(string_bounder);
        let sqrt5 = 5.0_f64.sqrt();
        let diagonal = (dim_title.width + 2.0 * dim_title.height) / sqrt5 + 2.0 * f64::from(MARGIN);
        XDimension2D::new(diagonal * sqrt5, diagonal * sqrt5 / 2.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self.calculate_dimension(ug.string_bounder());
        let dim_title = self.image.title.calculate_dimension(ug.string_bounder());
        let mut shapes = vec![(0.0, 0.0, diamond(dim_total))];
        if self.is_identifying {
            shapes.push((10.0, 5.0, diamond(dim_total.delta(-20.0, -10.0))));
        }
        let x_title = (dim_total.width - dim_title.width) / 2.0;
        let y_title = (dim_total.height - dim_title.height) / 2.0;
        self.image
            .draw(ug, self.image.style.stroke(), &shapes, (x_title, y_title));
    }

    fn backcolor(&self) -> Option<crate::color::HColor> {
        Some(self.image.base.get_backcolor())
    }
}

impl IEntityImage for EntityImageChenRelationship {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Diamond
    }

    fn is_hidden(&self) -> bool {
        self.image.base.is_hidden()
    }
}

/// Room above the title of attributes and circles.
const ELLIPSE_MARGIN: f64 = 6.0;

/// An ellipse, doubled for multi-valued attributes and dashed for derived ones.
struct ChenEllipse {
    image: ChenImage,
    is_multi: bool,
    is_derived: bool,
}

impl ChenEllipse {
    fn new(entity: &Entity, diagram: &CucaDiagram, element: SName) -> Self {
        Self {
            image: ChenImage::new(entity, diagram, element, has_stereotype(entity, "<<key>>")),
            is_multi: has_stereotype(entity, "<<multi>>"),
            is_derived: has_stereotype(entity, "<<derived>>"),
        }
    }

    fn draw(&self, ug: &UGraphic, dim_total: XDimension2D, y_title: f64) {
        let dim_title = self.image.title.calculate_dimension(ug.string_bounder());
        let mut stroke = self.image.style.stroke();
        if self.is_derived {
            stroke = UStroke {
                dash_visible: 10.0,
                dash_space: 10.0,
                thickness: stroke.thickness,
            };
        }
        let shape = |dim: XDimension2D| UShape::Ellipse(UEllipse::new(dim.width, dim.height));
        let mut shapes = vec![(0.0, 0.0, shape(dim_total))];
        if self.is_multi {
            shapes.push((3.0, 3.0, shape(dim_total.delta(-6.0, -6.0))));
        }
        let x_title = (dim_total.width - dim_title.width) / 2.0;
        self.image.draw(ug, stroke, &shapes, (x_title, y_title));
    }
}

/// A Chen attribute: an ellipse around its name, which is underlined for keys.
pub(crate) struct EntityImageChenAttribute(ChenEllipse);

impl EntityImageChenAttribute {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self(ChenEllipse::new(entity, diagram, SName::ChenAttribute))
    }
}

impl TextBlock for EntityImageChenAttribute {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        TextBlockInEllipse::new(&self.0.image.title, string_bounder)
            .calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self.calculate_dimension(ug.string_bounder());
        self.0.draw(ug, dim_total, ELLIPSE_MARGIN);
    }

    fn backcolor(&self) -> Option<crate::color::HColor> {
        Some(self.0.image.base.get_backcolor())
    }
}

impl IEntityImage for EntityImageChenAttribute {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Oval
    }

    fn is_hidden(&self) -> bool {
        self.0.image.base.is_hidden()
    }
}

/// The circle a superclass's subclasses hang from, with `d`, `o` or `U` in it.
pub(crate) struct EntityImageChenCircle(ChenEllipse);

impl EntityImageChenCircle {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        Self(ChenEllipse::new(entity, diagram, SName::Circle))
    }
}

impl TextBlock for EntityImageChenCircle {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(25.0, 25.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let dim_total = self.calculate_dimension(ug.string_bounder());
        let dim_title = self.0.image.title.calculate_dimension(ug.string_bounder());
        self.0
            .draw(ug, dim_total, (dim_total.height - dim_title.height) / 2.0);
    }

    fn backcolor(&self) -> Option<crate::color::HColor> {
        Some(self.0.image.base.get_backcolor())
    }
}

impl IEntityImage for EntityImageChenCircle {
    fn get_shape_type(&self) -> ShapeType {
        ShapeType::Oval
    }

    fn is_hidden(&self) -> bool {
        self.0.image.base.is_hidden()
    }
}
