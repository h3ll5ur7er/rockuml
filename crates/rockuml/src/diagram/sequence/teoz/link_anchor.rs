//! `{start} <-> {end} : text`: a double-headed vertical arrow between two anchored messages (the drawing
//! half of PlantUML's `LinkAnchor`, with the straight two-point `Snake` it draws).

use super::tile::Tile;
use crate::creole::Display;
use crate::diagram::sequence::LinkAnchor;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::styles::{merged, sequence_signature};
use crate::klimt::HorizontalAlignment;
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::component::creole_text;
use crate::style::{PName, SName, StyleSignature, ValueReading};

/// How far `ArrowsRegular`'s heads reach along the line, and to each side of it.
const DELTA1: f64 = 10.0;
const DELTA2: f64 = 4.0;

/// Draws the arrow between the contact lines of the two tiles, halfway between their middles.
pub(super) fn draw_anchor(
    link_anchor: &LinkAnchor,
    ug: &UGraphic,
    tile1: &dyn Tile<'_>,
    tile2: &dyn Tile<'_>,
    diagram: &SequenceDiagram,
) {
    let y1 = tile1.y_gauge().min.current_value() + tile1.contact_point_relative();
    let y2 = tile2.y_gauge().min.current_value() + tile2.contact_point_relative();
    let x = f64::midpoint(tile1.middle_x(), tile2.middle_x());
    let ymin = y1.min(y2);
    let ymax = y1.max(y2);

    let style_builder = diagram.style_builder();
    let style = merged(&style_builder, &sequence_signature(SName::Arrow));
    let color = style.value(PName::LineColor).as_color();
    let activity_arrow = merged(
        &style_builder,
        &StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::ActivityDiagram,
            SName::Activity,
            SName::Arrow,
        ]),
    );

    let (start, end) = (ymin + 2.0, ymax - 2.0);
    let line = ug
        .with_color(color.clone())
        .with_backcolor(color)
        .with_stroke(activity_arrow.stroke());
    line.translated(x, start).draw(&UShape::Line {
        dx: 0.0,
        dy: end - start,
    });
    let head = line.with_stroke(UStroke::SIMPLE);
    head.translated(x, start).draw(&as_to_up());
    head.translated(x, end).draw(&as_to_down());

    let Some(message) = &link_anchor.message else {
        return;
    };
    let display = Display::with_newlines(message);
    let title = creole_text(
        display.lines(),
        style.font_configuration(),
        display
            .natural_alignment()
            .unwrap_or(HorizontalAlignment::Center),
        0.0,
        diagram.skin(),
    );
    let dimension = title.calculate_dimension(ug.string_bounder());
    if dimension.width == 0.0 && dimension.height == 0.0 {
        return;
    }
    let text_y = f64::midpoint(start, end) - dimension.height / 2.0;
    title.draw_u(&ug.translated(x + 4.0, text_y));
}

fn as_to_up() -> UShape {
    UShape::Polygon(vec![
        (-DELTA2, DELTA1),
        (0.0, 0.0),
        (DELTA2, DELTA1),
        (0.0, DELTA1 - 4.0),
    ])
}

fn as_to_down() -> UShape {
    UShape::Polygon(vec![
        (-DELTA2, -DELTA1),
        (0.0, 0.0),
        (DELTA2, -DELTA1),
        (0.0, -DELTA1 + 4.0),
    ])
}
