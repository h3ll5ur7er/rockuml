//! A box drawn in perspective (PlantUML's `USymbolNode`). Separators in its text run on round its side.

use std::rc::Rc;

use super::{BigContent, BigShape, Margin, SmallShape};
use crate::color::HColor;
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::XDimension2D;
use crate::klimt::shape::UShape;
use crate::klimt::stencil::{HorizontalLineDrawer, UHorizontalLine};
use crate::klimt::ugraphic::UGraphic;

pub(super) struct USymbolNode;

const MARGIN: Margin = Margin::new(10.0 + 5.0, 20.0 + 5.0, 15.0 + 5.0, 5.0 + 5.0);

/// The empty square past the bottom left corner makes room for the box's depth.
fn draw_node(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(&UShape::Polygon(vec![
        (0.0, 10.0),
        (10.0, 0.0),
        (width, 0.0),
        (width, height - 10.0),
        (width - 10.0, height),
        (0.0, height),
        (0.0, 10.0),
    ]));
    ug.translated(width - 10.0, 10.0).draw(&UShape::Line {
        dx: 10.0,
        dy: -10.0,
    });
    ug.translated(0.0, 10.0).draw(&UShape::Line {
        dx: width - 10.0,
        dy: 0.0,
    });
    ug.translated(width - 10.0, 10.0).draw(&UShape::Line {
        dx: 0.0,
        dy: height - 10.0,
    });
    ug.translated(0.0, height)
        .draw(&UShape::Empty(XDimension2D::new(10.0, 10.0)));
}

/// Draws separators across the front and up the side (`MyUGraphicNode`).
struct NodeLines {
    ending_x: f64,
}

impl NodeLines {
    fn draw_hline_internal(&self, ug: &UGraphic, line: &UHorizontalLine) {
        let ug = ug.with_stroke(line.stroke()).with_backcolor(HColor::NONE);
        ug.draw(&UShape::Line {
            dx: self.ending_x - 10.0,
            dy: 0.0,
        });
        ug.translated(self.ending_x - 10.0, 0.0)
            .draw(&UShape::Line {
                dx: 10.0,
                dy: -10.0,
            });
    }
}

impl HorizontalLineDrawer for NodeLines {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, y: f64) {
        let ug = ug.translated(0.0, y);
        self.draw_hline_internal(&ug, line);
        if line.is_double() {
            self.draw_hline_internal(&ug.translated(0.0, 2.0), line);
        }
        line.draw_title_internal(&ug, 0.0, self.ending_x - 10.0, 0.0, true);
    }
}

impl SmallShape for USymbolNode {
    fn margin(&self) -> Margin {
        MARGIN
    }

    fn draw_shape(&self, ug: &UGraphic, dimension: XDimension2D, _fashion: &Fashion) {
        draw_node(ug, dimension.width, dimension.height);
    }

    fn text_alignment(&self, stereo_alignment: HorizontalAlignment) -> HorizontalAlignment {
        stereo_alignment
    }

    fn text_surface(&self, ug: &UGraphic, dimension: XDimension2D) -> UGraphic {
        ug.with_horizontal_line_drawer(Rc::new(NodeLines {
            ending_x: dimension.width,
        }))
    }
}

impl BigShape for USymbolNode {
    fn draw_big(&self, ug: &UGraphic, content: &BigContent) {
        draw_node(ug, content.width, content.height);
        let ug = ug.translated(-4.0, 11.0);
        let string_bounder = ug.string_bounder();
        let dim_stereo = content.stereotype.calculate_dimension(string_bounder);
        let pos_stereo_x = if content.stereo_alignment == HorizontalAlignment::Right {
            content.width - dim_stereo.width - MARGIN.x1
        } else {
            content.centered_x(dim_stereo.width)
        };
        content.stereotype.draw_u(&ug.translated(pos_stereo_x, 2.0));
        let dim_title = content.title.calculate_dimension(string_bounder);
        content
            .title
            .draw_u(&ug.translated(content.centered_x(dim_title.width), 2.0 + dim_stereo.height));
    }
}
