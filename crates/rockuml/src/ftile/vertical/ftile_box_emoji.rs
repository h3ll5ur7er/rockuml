//! An activity stereotyped `<<icon>>`: its leading emoji drawn large, the rest of its text beside it, no box
//! (PlantUML's `FtileBoxEmoji`).

use std::rc::Rc;

use crate::creole::{CreoleMode, Display, SheetBlock2, emoji_matching_size};
use crate::decoration::Rainbow;
use crate::diagram::activity3::{LinkRendering, SwimlaneId, SwimlaneSet};
use crate::ftile::{AbstractFtile, Ftile, FtileGeometry, Swimable};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{SName, StyleBuilder, StyleSignature};

const MARGIN: f64 = 5.0;

pub(crate) struct FtileBoxEmoji {
    base: AbstractFtile,
    emoji: SheetBlock2,
    /// The text after the emoji; none when the activity does not start with one.
    name: Option<SheetBlock2>,
    in_rendering: LinkRendering,
    swimlane: Option<SwimlaneId>,
}

impl FtileBoxEmoji {
    pub(crate) fn create(
        skin_param: Rc<SkinParam>,
        label: &Display,
        swimlane: Option<SwimlaneId>,
        stereotype: &Stereotype,
        style_builder: &StyleBuilder,
    ) -> Self {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
        ])
        .get_merged_style_with(style_builder, Some(stereotype));
        let style_arrow = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Arrow,
        ])
        .get_merged_style(style_builder);
        let fc = style.font_configuration();
        let text = |display: &Display| create_text(display, &fc, &skin_param);
        let first = label.lines().first().map_or("", String::as_str);
        let (emoji, name) = match emoji_matching_size(first) {
            0 => (text(label), None),
            position => {
                let (part1, part2) = first.split_at(position);
                (
                    text(&Display::create([eventually_hack_size(part1, "2")])),
                    Some(text(&Display::create([part2]))),
                )
            }
        };
        Self {
            base: AbstractFtile::new(skin_param),
            emoji,
            name,
            in_rendering: LinkRendering::create(Rainbow::build_from_style(&style_arrow)),
            swimlane,
        }
    }

    fn name_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.name
            .as_ref()
            .map_or_else(XDimension2D::default, |name| {
                name.calculate_dimension(string_bounder)
            })
    }
}

fn create_text(display: &Display, fc: &FontConfiguration, skin_param: &SkinParam) -> SheetBlock2 {
    display.create0(
        fc,
        HorizontalAlignment::Left,
        skin_param,
        0.0,
        CreoleMode::Full,
    )
}

/// The emoji twice as large, unless its size is written.
fn eventually_hack_size(part1: &str, size: &str) -> String {
    if part1.contains('*') {
        return part1.to_owned();
    }
    part1.replace(":>", &format!(":*{size}>"))
}

impl Swimable for FtileBoxEmoji {
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

impl Ftile for FtileBoxEmoji {
    fn skin_param(&self) -> &SkinParam {
        self.base.skin_param()
    }

    fn get_in_link_rendering(&self) -> LinkRendering {
        self.in_rendering.clone()
    }

    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> FtileGeometry {
        self.base.calculate_dimension(|| {
            let dim_emoji = self.emoji.calculate_dimension(string_bounder);
            let dim_name = self.name_dimension(string_bounder);
            let width = dim_emoji.width + MARGIN + dim_name.width;
            let height = dim_emoji.height.max(dim_name.height);
            let delta_y = (dim_emoji.height - dim_name.height) / 2.0;
            if delta_y > 0.0 {
                FtileGeometry::with_out(width, height, dim_emoji.width / 2.0, 0.0, dim_emoji.height)
            } else {
                FtileGeometry::with_out(
                    width,
                    height,
                    dim_emoji.width / 2.0,
                    -delta_y,
                    dim_emoji.height - delta_y,
                )
            }
        })
    }

    fn draw_u(&self, ug: &UGraphic) {
        let Some(name) = &self.name else {
            self.emoji.draw_u(ug);
            return;
        };
        let string_bounder = ug.string_bounder();
        let dim_emoji = self.emoji.calculate_dimension(string_bounder);
        let dim_name = name.calculate_dimension(string_bounder);
        let delta_x = dim_emoji.width + MARGIN;
        let delta_y = (dim_emoji.height - dim_name.height) / 2.0;
        if delta_y > 0.0 {
            self.emoji.draw_u(ug);
            name.draw_u(&ug.apply(UTranslate::new(delta_x, delta_y)));
        } else {
            self.emoji.draw_u(&ug.apply(UTranslate::new(0.0, -delta_y)));
            name.draw_u(&ug.apply(UTranslate::new(delta_x, 0.0)));
        }
    }
}
