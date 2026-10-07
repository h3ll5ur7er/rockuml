//! A package without content, drawn as a leaf in the big form of its package style (PlantUML's
//! `EntityImageEmptyPackage`).

use std::rc::Rc;

use crate::abel::{Entity, EntityPortion};
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, Display};
use crate::decoration::symbol::{Block, PackageStyle};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, RectangleArea, XDimension2D};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::component::TextBlockEmpty;
use crate::skin::font_param::FontParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::{ClusterDecoration, IEntityImage};

/// The room around the title and stereotype.
const MARGIN: f64 = 10.0;

pub(crate) struct EntityImageEmptyPackage {
    desc: Block,
    stereo_block: Block,
    url: Option<Url>,
    border_color: HColor,
    stroke: UStroke,
    round_corner: f64,
    diagonal_corner: f64,
    back: HColor,
    package_style: PackageStyle,
    title_alignment: HorizontalAlignment,
    stereotype_alignment: HorizontalAlignment,
}

impl EntityImageEmptyPackage {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let skin = diagram.skin();
        let colors = &entity.colors;
        let stereotype = entity.stereotype.as_ref();
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            diagram.get_style_name(),
            SName::Package,
            SName::Title,
        ])
        .get_merged_style_with(&skin.current_style_builder(), stereotype)
        .eventually_override_colors(colors);
        let title_horizontal_alignment = style.horizontal_alignment().unwrap_or_default();
        let desc = entity.display.create0(
            &style.font_configuration(),
            title_horizontal_alignment,
            skin,
            0.0,
            CreoleMode::Full,
        );
        let stereo_block: Block = match stereotype {
            Some(stereotype)
                if !stereotype.is_with_oo_symbol()
                    && diagram.show_portion(EntityPortion::Stereotype, entity.id()) =>
            {
                Rc::new(TextBlockMarged::new(
                    Display::create(stereotype.labels()).create0(
                        &skin
                            .get_font_configuration(FontParam::PackageStereotype, Some(stereotype)),
                        title_horizontal_alignment,
                        skin,
                        0.0,
                        CreoleMode::Full,
                    ),
                    ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 1.0, 0.0, 1.0),
                ))
            }
            _ => Rc::new(TextBlockEmpty::default()),
        };
        Self {
            desc: Rc::new(desc),
            stereo_block,
            url: entity.url.clone(),
            border_color: style.value(PName::LineColor).as_color(),
            stroke: style.stroke_with(colors),
            round_corner: style.value(PName::RoundCorner).as_double(),
            diagonal_corner: style.value(PName::DiagonalCorner).as_double(),
            back: colors
                .get(ColorType::Back)
                .cloned()
                .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            package_style: skin.package_style(),
            title_alignment: skin.package_title_alignment(),
            stereotype_alignment: skin.stereotype_alignment(),
        }
    }
}

impl TextBlock for EntityImageEmptyPackage {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_desc = self.desc.calculate_dimension(string_bounder);
        dim_desc
            .merge_top_bottom(self.stereo_block.calculate_dimension(string_bounder))
            .at_least(0.0, 2.0 * dim_desc.height)
            .delta(MARGIN * 2.0, MARGIN * 2.0)
    }

    fn draw_u(&self, ug: &UGraphic) {
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        let dim_total = self.calculate_dimension(ug.string_bounder());
        let rectangle_area = RectangleArea::new(0.0, 0.0, dim_total.width, dim_total.height);
        let decoration = ClusterDecoration::new(
            self.package_style,
            None,
            self.desc.clone(),
            self.stereo_block.clone(),
            rectangle_area,
            self.stroke,
        );
        decoration.draw_u(
            ug,
            self.back.clone(),
            self.border_color.clone(),
            self.round_corner,
            self.title_alignment,
            self.stereotype_alignment,
            self.diagonal_corner,
        );
        if self.url.is_some() {
            ug.close_url();
        }
    }
}

impl IEntityImage for EntityImageEmptyPackage {}
