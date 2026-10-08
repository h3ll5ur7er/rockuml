//! PlantUML's `FtileBlackBlock`: the bar a fork starts and joins at, with the join specification, like
//! `{or}`, beside it.

use std::rc::Rc;

use crate::color::Colors;
use crate::diagram::activity3::{SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::UTranslate;
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature};

const LABEL_MARGIN: f64 = 5.0;

pub(crate) struct FtileBlackBlock {
    base: AbstractFtile,
    width: f64,
    height: f64,
    colors: Colors,
    label: Option<Rc<dyn TextBlock>>,
    swimlane: Option<SwimlaneId>,
}

impl FtileBlackBlock {
    /// PlantUML sizes the bar and sets its label once the flows it spans are built; here the builder
    /// knows them before it builds the bar.
    pub(crate) fn new(
        skin_param: Rc<SkinParam>,
        swimlane: Option<SwimlaneId>,
        colors: Colors,
        width: f64,
        height: f64,
        label: Option<Rc<dyn TextBlock>>,
    ) -> Self {
        Self {
            base: AbstractFtile::new(skin_param),
            width,
            height,
            colors,
            label,
            swimlane,
        }
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let mut supp = self
            .label
            .as_ref()
            .map_or(0.0, |label| label.calculate_dimension(string_bounder).width);
        if supp > 0.0 {
            supp += LABEL_MARGIN;
        }
        FtileGeometry::with_out(
            self.width + supp,
            self.height,
            self.width / 2.0,
            0.0,
            self.height,
        )
    }

    fn get_signature() -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::ActivityBar,
        ])
    }
}

impl Swimable for FtileBlackBlock {
    fn get_swimlanes(&self) -> SwimlaneSet {
        SwimlaneSet::from([self.swimlane])
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.swimlane
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

impl Ftile for FtileBlackBlock {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let rect = URectangle::new(self.width, self.height)
            .rounded(5.0)
            .ignore_for_compression_on_x();
        let style = self
            .skin_param()
            .merged_style(&Self::get_signature())
            .expect("the skin styles activity bars");
        let color_bar = self.colors.get_color_of(&style, PName::BackGroundColor);
        ug.apply(color_bar.clone())
            .with_backcolor(color_bar)
            .draw(&UShape::Rectangle(rect));
        if let Some(label) = &self.label {
            let dim_label = label.calculate_dimension(ug.string_bounder());
            label.draw_u(&ug.apply(UTranslate::new(
                self.width + LABEL_MARGIN,
                -dim_label.height / 2.0,
            )));
        }
    }
}
