//! The box of a laid out node, cluster or label, in Graphviz coordinates (y up) (PlantUML's `BoxInfo`).
use smetana::{BoundingBox, Label, NodeLayout};

use crate::klimt::geom::XPoint2D;

pub(crate) struct BoxInfo {
    upper_right: XPoint2D,
    lower_left: XPoint2D,
}

impl BoxInfo {
    pub(crate) fn from_textlabel(label: &Label) -> Self {
        let (x, y) = (label.pos.x, label.pos.y);
        let (width, height) = (label.size.x, label.size.y);
        Self {
            upper_right: XPoint2D::new(x + width / 2.0, y - height / 2.0),
            lower_left: XPoint2D::new(x - width / 2.0, y + height / 2.0),
        }
    }

    pub(crate) fn from_node(node: &NodeLayout) -> Self {
        let width = node.width * 72.0;
        let height = node.height * 72.0;
        let (x, y) = (node.center.x, node.center.y);
        Self {
            upper_right: XPoint2D::new(x + width / 2.0, y - height / 2.0),
            lower_left: XPoint2D::new(x - width / 2.0, y + height / 2.0),
        }
    }

    pub(crate) fn from_graph_info(bb: &BoundingBox) -> Self {
        Self {
            upper_right: XPoint2D::new(bb.upper_right.x, bb.upper_right.y),
            lower_left: XPoint2D::new(bb.lower_left.x, bb.lower_left.y),
        }
    }

    pub(crate) fn get_upper_right(&self) -> XPoint2D {
        self.upper_right
    }

    pub(crate) fn get_lower_left(&self) -> XPoint2D {
        self.lower_left
    }
}
