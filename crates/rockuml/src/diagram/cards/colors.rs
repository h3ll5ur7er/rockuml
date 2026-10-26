//! The colour chart, `colors`: every colour name PlantUML knows, or the colours closest to one, on
//! hexagons around it.

use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use super::{Card, UgDiagram};
use crate::color::{HColor, XColor, color_names};
use crate::creole::{CreoleMode, Display, SheetBlock2};
use crate::diagram::builder::{PSystemSingleLineFactory, whole_line};
use crate::diagram::{Diagram, NotYetPorted, UmlSource};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::shape::{UPolygon, URectangle, UShape};
use crate::klimt::sprite::SpriteContainerEmpty;
use crate::klimt::typeface::FontRegistry;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};

/// `colors`, or `colors` and the colour to centre the chart on (PlantUML's `PSystemColorsFactory`).
pub(in crate::diagram) struct PSystemColorsFactory;

impl PSystemSingleLineFactory for PSystemColorsFactory {
    fn execute_line(
        source: &Rc<UmlSource>,
        line: &str,
    ) -> Option<Result<Box<dyn Diagram>, NotYetPorted>> {
        static PATTERN: LazyLock<Regex> = LazyLock::new(|| whole_line(r"colors?\s*(#?\w+)?\s*"));
        let captures = PATTERN.captures(line)?;
        let option = captures.get(1).map(|option| option.as_str());
        Some(Ok(UgDiagram::boxed(source, PSystemColors::new(option))))
    }
}

/// The hexagons of the chart around a colour, by their place in the hexagonal grid
/// (<http://www.redblobgames.com/grids/hexagons/>), the closest colours first.
const HEXAGONS: [(i32, i32); 19] = [
    (0, 0),
    (1, 0),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (2, 0),
    (1, 1),
    (1, 2),
    (0, 2),
    (-1, 2),
    (-2, 1),
    (-2, 0),
    (-2, -1),
    (-1, -2),
    (0, -2),
    (1, -2),
    (1, -1),
];

/// PlantUML's `PSystemColors`.
struct PSystemColors {
    palette_central_color: Option<String>,
}

impl PSystemColors {
    const RECTANGLE_HEIGHT: f64 = 28.0;
    const RECTANGLE_WIDTH: f64 = 175.0;
    const SIZE: f64 = 60.0;
    const NB_ROWS: usize = 21;

    fn new(option: Option<&str>) -> Self {
        Self {
            palette_central_color: option.map(|option| option.replace('#', "")),
        }
    }

    /// The colour the chart centres on, unless it names none that is one colour.
    fn palette_center(&self) -> Option<(&str, XColor)> {
        let name = self.palette_central_color.as_deref()?;
        match HColor::parse_or_white(name) {
            HColor::Simple(color) => Some((name, color)),
            _ => None,
        }
    }

    fn calculate_dimension_full() -> XDimension2D {
        let nb_columns = color_names().len().div_ceil(Self::NB_ROWS);
        XDimension2D::new(
            nb_columns as f64 * Self::RECTANGLE_WIDTH,
            Self::NB_ROWS as f64 * Self::RECTANGLE_HEIGHT,
        )
    }

    fn calculate_dimension_palette() -> XDimension2D {
        let (translate_x, translate_y) = Self::palette_origin();
        let max_center_x = Self::width() * 2.0 + Self::width() / 2.0;
        let max_center_y = Self::SIZE * 2.0 * 1.5;
        XDimension2D::new(
            translate_x + max_center_x + Self::SIZE + 1.0,
            translate_y + max_center_y + Self::SIZE + 1.0,
        )
    }

    /// Where the central hexagon goes, so that the others fit.
    fn palette_origin() -> (f64, f64) {
        (
            f64::midpoint(center_hexa(2, 0).x, center_hexa(3, 0).x),
            center_hexa(0, 2).y + corner(1).y,
        )
    }

    /// The width of a hexagon.
    fn width() -> f64 {
        3.0_f64.sqrt() / 2.0 * 2.0 * Self::SIZE
    }

    fn draw_palette(ug: &UGraphic, center_name: &str, center: XColor) {
        let (x, y) = Self::palette_origin();
        let ug = ug.translated(x, y);
        let hexa = UShape::Polygon(UPolygon::new(
            (0..6).map(corner).map(|point| (point.x, point.y)).collect(),
        ));
        let friends = colors_close_to(center_name, center);
        for (name, (i, j)) in friends.iter().zip(HEXAGONS) {
            draw_one_hexa(&ug, name, i, j, &hexa);
        }
    }

    fn draw_full(ug: &UGraphic) {
        let ug = ug.with_color(HColor::BLACK);
        for (index, name) in color_names().into_iter().enumerate() {
            let i = index / Self::NB_ROWS;
            let j = index % Self::NB_ROWS;
            let tmp = ug.translated(
                Self::RECTANGLE_WIDTH * i as f64,
                Self::RECTANGLE_HEIGHT * j as f64,
            );
            let color = simple_color(name);
            apply_color(&tmp, color).draw(&UShape::Rectangle(URectangle::new(
                Self::RECTANGLE_WIDTH,
                Self::RECTANGLE_HEIGHT,
            )));
            let tt = get_text_name(name, color);
            let dim_text = tt.calculate_dimension(ug.string_bounder());
            let dy = (Self::RECTANGLE_HEIGHT - dim_text.height) / 2.0;
            let dx = (Self::RECTANGLE_WIDTH - dim_text.width) / 2.0;
            tt.draw_u(&tmp.translated(dx, dy));
        }
    }
}

impl Card for PSystemColors {
    fn get_text_block(&self, _fonts: &FontRegistry) -> Box<dyn TextBlock + '_> {
        Box::new(self)
    }
}

impl TextBlock for PSystemColors {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        match self.palette_center() {
            Some(_) => Self::calculate_dimension_palette(),
            None => Self::calculate_dimension_full(),
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        match self.palette_center() {
            Some((name, color)) => Self::draw_palette(ug, name, color),
            None => Self::draw_full(ug),
        }
    }
}

fn center_hexa(i: i32, j: i32) -> XPoint2D {
    let width = PSystemColors::width();
    let x = width * f64::from(i) + if j % 2 == 0 { 0.0 } else { width / 2.0 };
    let y = PSystemColors::SIZE * f64::from(j) * 1.5;
    XPoint2D::new(x, y)
}

fn corner(i: i32) -> XPoint2D {
    let angle_deg = f64::from(60 * i + 30);
    let angle_rad = std::f64::consts::PI / 180.0 * angle_deg;
    XPoint2D::new(
        PSystemColors::SIZE * libm::cos(angle_rad),
        PSystemColors::SIZE * libm::sin(angle_rad),
    )
}

fn draw_one_hexa(ug: &UGraphic, color_name: &str, i: i32, j: i32, hexa: &UShape) {
    let color = simple_color(color_name);
    let center = center_hexa(i, j);
    let ug = apply_color(ug, color).translated(center.x, center.y);
    ug.draw(hexa);
    let mut tt = get_text_name(color_name, color);
    let mut dim_text = tt.calculate_dimension(ug.string_bounder());
    if dim_text.width > PSystemColors::width() {
        // PlantUML draws nothing of a long name it cannot cut before a capital.
        tt = match find_shortest(ug.string_bounder(), color_name) {
            Some(shortest) => get_text_name(&shortest, color),
            None => create_text_name(&Display::create(Vec::<String>::new()), color),
        };
        dim_text = tt.calculate_dimension(ug.string_bounder());
    }
    tt.draw_u(&ug.translated(-dim_text.width / 2.0, -dim_text.height / 2.0));
}

/// The name cut on two lines before one of its capitals, where it is the narrowest.
fn find_shortest(string_bounder: &dyn StringBounder, color_name: &str) -> Option<String> {
    let chars: Vec<char> = color_name.chars().collect();
    let mut result = None;
    let mut min = f64::MAX;
    for i in 1..chars.len().saturating_sub(1) {
        if chars[i].is_lowercase() {
            continue;
        }
        let head: String = chars[..i].iter().collect();
        let tail: String = chars[i..].iter().collect();
        let candidate = format!("{head}\\n{tail}");
        let width = get_text_name(&candidate, XColor::rgb(0, 0, 0))
            .calculate_dimension(string_bounder)
            .width;
        if width < min {
            result = Some(candidate);
            min = width;
        }
    }
    result
}

fn apply_color(ug: &UGraphic, color: XColor) -> UGraphic {
    ug.with_color(HColor::Simple(color))
        .with_backcolor(HColor::Simple(color))
}

/// The colour of a name the chart shows, which is always one colour.
fn simple_color(name: &str) -> XColor {
    match HColor::parse_or_white(name) {
        HColor::Simple(color) => color,
        _ => unreachable!("the chart shows colour names and the colour it centres on"),
    }
}

/// Every colour name but the `Gray` spellings of `Grey` ones, and `other`, from the closest to `center` on.
fn colors_close_to(other: &str, center: XColor) -> Vec<String> {
    let names = color_names();
    let mut result: Vec<String> = names
        .iter()
        .filter(|candidate| {
            let similar = candidate.replace("Gray", "Grey");
            similar == **candidate || !names.contains(&similar.as_str())
        })
        .map(|name| (*name).to_owned())
        .collect();
    if !result.iter().any(|name| name.eq_ignore_ascii_case(other)) {
        result.push(other.to_owned());
    }
    result.sort_by_key(|name| center.distance(simple_color(name)));
    result
}

/// The name in bold, black or white whichever reads better on the colour.
fn get_text_name(name: &str, color: XColor) -> SheetBlock2 {
    create_text_name(&Display::with_newlines(name), color)
}

fn create_text_name(display: &Display, color: XColor) -> SheetBlock2 {
    let font = FontConfiguration::new(
        UFont::new("SansSerif", UFontFace::BOLD, 14),
        HColor::Simple(color.opposite()),
        8,
    );
    display.create0(
        &font,
        HorizontalAlignment::Center,
        &SpriteContainerEmpty,
        0.0,
        CreoleMode::Full,
    )
}
