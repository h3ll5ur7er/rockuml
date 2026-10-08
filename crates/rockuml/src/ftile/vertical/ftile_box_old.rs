//! A node of a mind map or a work breakdown: its text in a box, outlined as its style says (PlantUML's
//! `FtileBoxOld`, which only those diagrams still use, as a text block).

use std::rc::Rc;

use crate::color::{Colors, HColor};
use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1, SheetBlock2};
use crate::ftile::BoxStyle;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::stencil::Stencil;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, Style, ValueReading};

pub(crate) struct FtileBoxOld {
    tb: SheetBlock2,
    sheet: Rc<SheetBlock1>,
    round_corner: f64,
    horizontal_alignment: Option<HorizontalAlignment>,
    minimum_width: f64,
    box_style: BoxStyle,
    border_color: HColor,
    back_color: HColor,
    stroke: UStroke,
}

impl FtileBoxOld {
    /// `colors` are the node's own (PlantUML hands them over as a `SkinParamColors`).
    pub(crate) fn create_mind_map(
        style: &Style,
        skin_param: &SkinParam,
        colors: &Colors,
        label: &Display,
    ) -> Self {
        Self::new(skin_param, label, BoxStyle::Plain, style, colors)
    }

    fn new(
        skin_param: &SkinParam,
        label: &Display,
        box_style: BoxStyle,
        style: &Style,
        colors: &Colors,
    ) -> Self {
        let style = style.eventually_override_colors(colors);
        let fc = style.font_configuration();
        let horizontal_alignment = style.horizontal_alignment();
        let minimum_width = style.value(PName::MinimumWidth).as_double();
        let sheet = CreoleParser::with_mode(
            fc.clone(),
            horizontal_alignment.unwrap_or(HorizontalAlignment::Left),
            CreoleMode::Full,
            skin_param,
        )
        .create_display_sheet(label, &fc);
        let sheet =
            Rc::new(SheetBlock1::new(sheet, style.padding()).wrapped_at(style.wrap_width()));
        let stencil = Rc::new(BoxStencil {
            sheet: sheet.clone(),
            minimum_width,
        });
        Self {
            tb: SheetBlock2::with_stencil(sheet.clone(), stencil, UStroke::with_thickness(1.0)),
            sheet,
            round_corner: style.value(PName::RoundCorner).as_double(),
            horizontal_alignment,
            minimum_width,
            box_style,
            border_color: style.value(PName::LineColor).as_color(),
            back_color: style.value(PName::BackGroundColor).as_color(),
            stroke: style.stroke(),
        }
    }

    fn tb_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.minimum_width
            .max(self.tb.calculate_dimension(string_bounder).width)
    }
}

/// Separators of the text span the box (`MyStencil`), which is as wide as the text but at least its minimum
/// width.
struct BoxStencil {
    sheet: Rc<SheetBlock1>,
    minimum_width: f64,
}

impl Stencil for BoxStencil {
    fn starting_x(&self, _string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        0.0
    }

    fn ending_x(&self, string_bounder: &dyn StringBounder, _y: f64) -> f64 {
        self.sheet
            .calculate_dimension(string_bounder)
            .width
            .max(self.minimum_width)
    }
}

impl TextBlock for FtileBoxOld {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_raw = self
            .sheet
            .calculate_dimension(string_bounder)
            .at_least(self.minimum_width, 0.0);
        XDimension2D::new(dim_raw.width + self.box_style.get_shield(), dim_raw.height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let dim_total = self.calculate_dimension(string_bounder);
        let ug = ug
            .apply(self.border_color.clone())
            .with_backcolor(self.back_color.clone())
            .apply(self.stroke);
        self.box_style
            .draw_me(&ug, dim_total.width, dim_total.height, self.round_corner);
        // PlantUML draws no text for an alignment it does not know.
        match self.horizontal_alignment {
            Some(HorizontalAlignment::Left) => self.tb.draw_u(&ug),
            Some(HorizontalAlignment::Right) => self
                .tb
                .draw_u(&ug.translated(dim_total.width - self.tb_width(string_bounder), 0.0)),
            Some(HorizontalAlignment::Center) => self.tb.draw_u(
                &ug.translated((dim_total.width - self.tb_width(string_bounder)) / 2.0, 0.0),
            ),
            None => {}
        }
    }
}
