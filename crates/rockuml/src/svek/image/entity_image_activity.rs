//! An action of a legacy activity diagram: its text in a rounded box (PlantUML's `EntityImageActivity`).

use crate::abel::Entity;
use crate::color::{ColorType, HColor};
use crate::creole::{CreoleMode, SheetBlock2};
use crate::diagram::cuca::CucaDiagram;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::url::Url;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::IEntityImage;

const CORNER: f64 = 25.0;
const MARGIN: f64 = 10.0;

pub(crate) struct EntityImageActivity {
    desc: SheetBlock2,
    url: Option<Url>,
    border: HColor,
    back: HColor,
    stroke: UStroke,
    shadowing: f64,
}

impl EntityImageActivity {
    pub(crate) fn new(entity: &Entity, diagram: &CucaDiagram) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
        ])
        .get_merged_style_with(
            &diagram.skin().current_style_builder(),
            entity.stereotype.as_ref(),
        );
        let desc = entity.display.create0(
            &style.font_configuration(),
            style.horizontal_alignment().unwrap_or_default(),
            diagram.skin(),
            0.0,
            CreoleMode::Full,
        );
        Self {
            desc,
            url: entity.url.clone(),
            border: style.value(PName::LineColor).as_color(),
            back: entity
                .colors
                .get(ColorType::Back)
                .cloned()
                .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            stroke: style.stroke(),
            shadowing: style.shadowing(),
        }
    }
}

impl TextBlock for EntityImageActivity {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.desc
            .calculate_dimension(string_bounder)
            .delta(MARGIN * 2.0, MARGIN * 2.0)
    }

    /// `skinparam activityShape octagon` draws the octagon Graphviz gives the node; Smetana gives none, so
    /// PlantUML draws the rounded box then too.
    fn draw_u(&self, ug: &UGraphic) {
        if let Some(url) = &self.url {
            ug.start_url(url);
        }
        let dim_total = self.calculate_dimension(ug.string_bounder());
        let rect = URectangle::new(dim_total.width, dim_total.height)
            .rounded(CORNER)
            .with_delta_shadow(self.shadowing);
        let ug = ug
            .with_color(self.border.clone())
            .with_backcolor(self.back.clone());
        ug.with_stroke(self.stroke).draw(&UShape::Rectangle(rect));
        self.desc.draw_u(&ug.translated(MARGIN, MARGIN));
        if self.url.is_some() {
            ug.close_url();
        }
    }
}

impl IEntityImage for EntityImageActivity {}
