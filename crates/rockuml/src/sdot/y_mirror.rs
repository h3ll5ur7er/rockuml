//! Turns Graphviz coordinates (y up) into drawing coordinates (y down) (PlantUML's `YMirror`).

use crate::klimt::dot_path::DotPath;
use crate::klimt::geom::{UTranslate, XCubicCurve2D, XPoint2D};

#[derive(Clone, Copy)]
pub(crate) struct YMirror {
    max: f64,
}

impl YMirror {
    pub(crate) fn new(max: f64) -> Self {
        Self { max }
    }

    pub(crate) fn get_mirrored(self, pt: XPoint2D) -> XPoint2D {
        XPoint2D::new(pt.x, self.max - pt.y)
    }

    pub(crate) fn get_mirrored_path(self, path: &DotPath) -> DotPath {
        path.get_beziers()
            .iter()
            .fold(DotPath::default(), |result, bez| {
                result.add_curve(XCubicCurve2D::new(
                    self.get_mirrored(bez.get_p1()),
                    self.get_mirrored(bez.get_ctrl_p1()),
                    self.get_mirrored(bez.get_ctrl_p2()),
                    self.get_mirrored(bez.get_p2()),
                ))
            })
    }

    pub(crate) fn get_mirrored_translate(self, tr: UTranslate) -> UTranslate {
        UTranslate::new(tr.dx, self.max - tr.dy)
    }
}
