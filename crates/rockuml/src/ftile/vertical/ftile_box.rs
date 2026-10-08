//! An activity: its text in a box, outlined as its style says (PlantUML's `FtileBox`).

use std::rc::Rc;

use crate::color::{Colors, HColor};
use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, BoxStyle, Ftile, FtileGeometry, Swimable};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, UTranslate};
use crate::klimt::stencil::Stencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, StyleBuilder, StyleSignature, ValueReading};

pub(crate) struct FtileBox {
    base: AbstractFtile,
    padding: ClockwiseTopRightBottomLeft,
    tb: SheetBlock1,
    round_corner: f64,
    horizontal_alignment: Option<HorizontalAlignment>,
    minimum_width: f64,
    in_rendering: LinkRendering,
    swimlane: Option<SwimlaneId>,
    box_style: BoxStyle,
    border_color: HColor,
    back_color: HColor,
    /// The style's, which the activity's own colours do not change.
    stroke: UStroke,
}

impl FtileBox {
    fn get_style_signature() -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
        ])
    }

    fn get_style_signature_arrow() -> StyleSignature {
        StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
    }

    /// A box in the style `style_builder` gives activities with the stereotype, or else the box style's;
    /// `colors` are the activity's own (PlantUML hands them over as a `SkinParamColors`).
    pub(crate) fn create(
        skin_param: Rc<SkinParam>,
        colors: &Colors,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        box_style: BoxStyle,
        stereotype: Option<&Stereotype>,
        style_builder: &StyleBuilder,
    ) -> Self {
        let box_stereotype = box_style.get_stereotype();
        let stereotype = stereotype.or(box_stereotype.as_ref());
        let style = Self::get_style_signature().get_merged_style_with(style_builder, stereotype);
        let style_arrow = Self::get_style_signature_arrow().get_merged_style(style_builder);
        let in_rendering = LinkRendering::create(Rainbow::build_from_style(&style_arrow));
        let colored = style.eventually_override_colors(colors);
        let fc = colored.font_configuration();
        let horizontal_alignment = colored.horizontal_alignment();
        let sheet = CreoleParser::with_mode(
            fc.clone(),
            skin_param.get_default_text_alignment(
                horizontal_alignment.unwrap_or(HorizontalAlignment::Left),
            ),
            CreoleMode::Full,
            skin_param.as_ref(),
        )
        .create_display_sheet(label, &fc);
        Self {
            padding: colored.padding(),
            tb: SheetBlock1::new(sheet, skin_param.get_padding()).wrapped_at(colored.wrap_width()),
            round_corner: colored.value(PName::RoundCorner).as_double(),
            horizontal_alignment,
            minimum_width: colored.value(PName::MinimumWidth).as_double(),
            in_rendering,
            swimlane,
            box_style,
            border_color: colored.value(PName::LineColor).as_color(),
            back_color: colored.value(PName::BackGroundColor).as_color(),
            stroke: style.stroke(),
            base: AbstractFtile::new(skin_param),
        }
    }

    fn calculate_dimension_ftile(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        let dim_raw = self
            .tb
            .calculate_dimension(string_bounder)
            .delta(
                self.padding.left + self.padding.right,
                self.padding.bottom + self.padding.top,
            )
            .at_least(self.minimum_width, 0.0);
        FtileGeometry::with_out(
            dim_raw.width + self.box_style.get_shield(),
            dim_raw.height,
            dim_raw.width / 2.0,
            0.0,
            dim_raw.height,
        )
    }

    /// Draws the text at `translate`, its separators spanning the box less its padding (`MyStencil`).
    fn draw_text(&self, ug: &UGraphic, translate: UTranslate, box_width: f64) {
        let stencil = BoxStencil {
            starting_x: -self.padding.left,
            ending_x: box_width - self.padding.right,
        };
        self.tb.draw_u(
            &ug.apply(translate)
                .with_stencil_stroke(Rc::new(stencil), UStroke::with_thickness(1.0)),
        );
    }
}

/// Separators of the text span the box, whatever the text's width.
struct BoxStencil {
    starting_x: f64,
    ending_x: f64,
}

impl Stencil for BoxStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.starting_x
    }

    fn ending_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.ending_x
    }
}

impl Swimable for FtileBox {
    fn get_swimlanes(&self) -> SwimlaneSet {
        self.swimlane.map(Some).into_iter().collect()
    }

    fn get_swimlane_in(&self) -> Option<SwimlaneId> {
        self.swimlane
    }

    fn get_swimlane_out(&self) -> Option<SwimlaneId> {
        self.swimlane
    }
}

impl Ftile for FtileBox {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.in_rendering.clone()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base
            .calculate_dimension(|| self.calculate_dimension_ftile(string_bounder))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let ug = ug
            .apply(self.border_color.clone())
            .with_backcolor(self.back_color.clone())
            .apply(self.stroke);
        self.box_style.draw_me(
            &ug,
            dim_total.get_width(),
            dim_total.get_height(),
            self.round_corner,
        );
        let translate = match self.horizontal_alignment {
            Some(HorizontalAlignment::Left) => UTranslate::new(self.padding.left, self.padding.top),
            Some(HorizontalAlignment::Right) => {
                let dim_tb = self.tb.calculate_dimension(string_bounder);
                UTranslate::new(
                    dim_total.get_width() - dim_tb.width - self.padding.right,
                    self.padding.bottom,
                )
            }
            Some(HorizontalAlignment::Center) => {
                let dim_tb = self.tb.calculate_dimension(string_bounder);
                UTranslate::new(
                    (dim_total.get_width() - dim_tb.width) / 2.0,
                    self.padding.bottom,
                )
            }
            None => return,
        };
        self.draw_text(&ug, translate, dim_total.get_width());
    }
}
