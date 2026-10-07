//! A note's outline: a box with its top right corner folded, which may reach out to a point like a callout
//! (PlantUML's `Opale`).

use crate::color::HColor;
use crate::direction::Direction;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{XDimension2D, XPoint2D};
use crate::klimt::shape::{USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};

/// The size of the folded corner.
const CORNERSIZE: f64 = 10.0;
pub(crate) const MARGIN_X1: f64 = 6.0;
pub(crate) const MARGIN_X2: f64 = 15.0;
pub(crate) const MARGIN_Y: f64 = 5.0;
/// Half the width of the callout where it leaves the box.
const DELTA: f64 = 4.0;

/// The fold of the corner (`Opale.getCorner`).
pub(crate) fn get_corner(width: f64, round_corner: f64) -> Vec<USegment> {
    let mut path = vec![USegment::MoveTo(width - CORNERSIZE, 0.0)];
    if round_corner == 0.0 {
        path.push(USegment::LineTo(width - CORNERSIZE, CORNERSIZE));
    } else {
        path.push(USegment::LineTo(
            width - CORNERSIZE,
            CORNERSIZE - round_corner / 4.0,
        ));
        path.push(arc_to(
            (width - CORNERSIZE + round_corner / 4.0, CORNERSIZE),
            round_corner / 4.0,
        ));
    }
    path.push(USegment::LineTo(width, CORNERSIZE));
    path.push(USegment::LineTo(width - CORNERSIZE, 0.0));
    path
}

/// The outline without callout (`Opale.getPolygonNormal`).
pub(crate) fn get_polygon_normal(width: f64, height: f64, round_corner: f64) -> Vec<USegment> {
    if round_corner == 0.0 {
        return vec![
            USegment::MoveTo(0.0, 0.0),
            USegment::LineTo(0.0, height),
            USegment::LineTo(width, height),
            USegment::LineTo(width, CORNERSIZE),
            USegment::LineTo(width - CORNERSIZE, 0.0),
            USegment::LineTo(0.0, 0.0),
        ];
    }
    let half = round_corner / 2.0;
    vec![
        USegment::MoveTo(0.0, half),
        USegment::LineTo(0.0, height - half),
        arc_to((half, height), half),
        USegment::LineTo(width - half, height),
        arc_to((width, height - half), half),
        USegment::LineTo(width, CORNERSIZE),
        USegment::LineTo(width - CORNERSIZE, 0.0),
        USegment::LineTo(half, 0.0),
        arc_to((0.0, half), half),
    ]
}

/// `UPath.arcTo(pt, radius, 0, 0)`.
fn arc_to(end: (f64, f64), radius: f64) -> USegment {
    USegment::ArcTo {
        radius: (radius, radius),
        x_axis_rotation: 0.0,
        large_arc: false,
        sweep: false,
        end,
    }
}

/// `MathUtils.limitation`: `v` brought between `min` and `max`, unless they leave no room.
fn limitation(v: f64, min: f64, max: f64) -> f64 {
    if min >= max { v } else { v.clamp(min, max) }
}

/// A note's text in an outline with a callout towards a point.
pub(crate) struct Opale<'a> {
    note_background_color: HColor,
    border_color: HColor,
    text_block: Box<dyn TextBlock + 'a>,
    stroke: UStroke,
    round_corner: f64,
}

impl<'a> Opale<'a> {
    pub(crate) fn new(
        border_color: HColor,
        note_background_color: HColor,
        text_block: Box<dyn TextBlock + 'a>,
        stroke: UStroke,
        round_corner: f64,
    ) -> Self {
        Self {
            note_background_color,
            border_color,
            text_block,
            stroke,
            round_corner,
        }
    }

    fn get_width(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(string_bounder).width + MARGIN_X1 + MARGIN_X2
    }

    fn get_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(string_bounder).height + 2.0 * MARGIN_Y
    }

    pub(crate) fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.get_width(string_bounder),
            self.get_height(string_bounder),
        )
    }

    /// Draws the note with its callout leaving the side `strategy` names near `pp1` and reaching `pp2`
    /// (`setOpale`, then `drawU`).
    pub(crate) fn draw_u(&self, ug: &UGraphic, strategy: Direction, pp1: XPoint2D, pp2: XPoint2D) {
        let string_bounder = ug.string_bounder();
        let ug = ug
            .with_backcolor(self.note_background_color.clone())
            .with_color(self.border_color.clone())
            .with_stroke(self.stroke);
        let polygon = match strategy {
            Direction::Left => self.get_polygon_left(string_bounder, pp1, pp2),
            Direction::Right => self.get_polygon_right(string_bounder, pp1, pp2),
            Direction::Up => self.get_polygon_up(string_bounder, pp1, pp2),
            Direction::Down => self.get_polygon_down(string_bounder, pp1, pp2),
        };
        ug.draw(&UShape::Path(polygon));
        ug.draw(&UShape::Path(get_corner(
            self.get_width(string_bounder),
            self.round_corner,
        )));
        self.text_block.draw_u(&ug.translated(MARGIN_X1, MARGIN_Y));
    }

    /// The bottom left corner, the bottom, and the bottom right corner.
    fn bottom(&self, width: f64, height: f64, path: &mut Vec<USegment>) {
        let half = self.round_corner / 2.0;
        path.push(USegment::LineTo(0.0, height - half));
        path.push(arc_to((half, height), half));
        path.push(USegment::LineTo(width - half, height));
        path.push(arc_to((width, height - half), half));
    }

    /// From the right side down to the fold, the top, and the top left corner.
    fn top(&self, width: f64, path: &mut Vec<USegment>) {
        let half = self.round_corner / 2.0;
        path.push(USegment::LineTo(width, CORNERSIZE));
        path.push(USegment::LineTo(width - CORNERSIZE, 0.0));
        path.push(USegment::LineTo(half, 0.0));
        path.push(arc_to((0.0, half), half));
    }

    fn get_polygon_left(
        &self,
        string_bounder: &dyn StringBounder,
        pp1: XPoint2D,
        pp2: XPoint2D,
    ) -> Vec<USegment> {
        let (width, height) = (
            self.get_width(string_bounder),
            self.get_height(string_bounder),
        );
        let y1 = limitation(pp1.y - DELTA, 0.0, height - 2.0 * DELTA);
        let mut path = vec![
            USegment::MoveTo(0.0, self.round_corner / 2.0),
            USegment::LineTo(0.0, y1),
            USegment::LineTo(pp2.x, pp2.y),
            USegment::LineTo(0.0, y1 + 2.0 * DELTA),
        ];
        self.bottom(width, height, &mut path);
        self.top(width, &mut path);
        path
    }

    fn get_polygon_right(
        &self,
        string_bounder: &dyn StringBounder,
        pp1: XPoint2D,
        pp2: XPoint2D,
    ) -> Vec<USegment> {
        let (width, height) = (
            self.get_width(string_bounder),
            self.get_height(string_bounder),
        );
        let mut path = vec![USegment::MoveTo(0.0, self.round_corner / 2.0)];
        self.bottom(width, height, &mut path);
        let y1 = limitation(pp1.y - DELTA, CORNERSIZE, height - 2.0 * DELTA);
        path.extend([
            USegment::LineTo(width, y1 + 2.0 * DELTA),
            USegment::LineTo(pp2.x, pp2.y),
            USegment::LineTo(width, y1),
        ]);
        self.top(width, &mut path);
        path
    }

    fn get_polygon_up(
        &self,
        string_bounder: &dyn StringBounder,
        pp1: XPoint2D,
        pp2: XPoint2D,
    ) -> Vec<USegment> {
        let (width, height) = (
            self.get_width(string_bounder),
            self.get_height(string_bounder),
        );
        let half = self.round_corner / 2.0;
        let mut path = vec![USegment::MoveTo(0.0, half)];
        self.bottom(width, height, &mut path);
        let x1 = limitation(pp1.x - DELTA, 0.0, width - CORNERSIZE);
        path.extend([
            USegment::LineTo(width, CORNERSIZE),
            USegment::LineTo(width - CORNERSIZE, 0.0),
            USegment::LineTo(x1 + 2.0 * DELTA, 0.0),
            USegment::LineTo(pp2.x, pp2.y),
            USegment::LineTo(x1, 0.0),
            USegment::LineTo(half, 0.0),
            arc_to((0.0, half), half),
        ]);
        path
    }

    fn get_polygon_down(
        &self,
        string_bounder: &dyn StringBounder,
        pp1: XPoint2D,
        pp2: XPoint2D,
    ) -> Vec<USegment> {
        let (width, height) = (
            self.get_width(string_bounder),
            self.get_height(string_bounder),
        );
        let half = self.round_corner / 2.0;
        let x1 = limitation(pp1.x - DELTA, 0.0, width);
        let mut path = vec![
            USegment::MoveTo(0.0, half),
            USegment::LineTo(0.0, height - half),
            arc_to((half, height), half),
            USegment::LineTo(x1, height),
            USegment::LineTo(pp2.x, pp2.y),
            USegment::LineTo(x1 + 2.0 * DELTA, height),
            USegment::LineTo(width - half, height),
            arc_to((width, height - half), half),
        ];
        self.top(width, &mut path);
        path
    }
}
