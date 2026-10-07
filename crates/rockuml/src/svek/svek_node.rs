//! An entity laid out as a node (PlantUML's `SvekNode`, without what only Graphviz layouts need).

use std::cell::Cell;

use super::{ColorSequence, IEntityImage};
use crate::abel::EntityId;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{RectangleArea, UTranslate, XDimension2D, XPoint2D};

pub(crate) struct SvekNode {
    leaf: EntityId,
    dim_image: XDimension2D,
    uid: String,
    image: Box<dyn IEntityImage>,
    /// Where the layout put the node's top left corner; drawing moves it.
    min_x: Cell<f64>,
    min_y: Cell<f64>,
}

impl SvekNode {
    pub(super) fn new(
        leaf: EntityId,
        image: Box<dyn IEntityImage>,
        color_sequence: &mut ColorSequence,
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let color = color_sequence.get_value();
        Self {
            leaf,
            dim_image: image.calculate_dimension(string_bounder),
            uid: format!("sh{color:04}"),
            image,
            min_x: Cell::new(0.0),
            min_y: Cell::new(0.0),
        }
    }

    pub(crate) fn get_leaf(&self) -> EntityId {
        self.leaf
    }

    /// `sh0006`: the name of the node in the layout.
    pub(crate) fn get_uid(&self) -> &str {
        &self.uid
    }

    pub(crate) fn get_image(&self) -> &dyn IEntityImage {
        self.image.as_ref()
    }

    pub(crate) fn get_image_mut(&mut self) -> &mut dyn IEntityImage {
        self.image.as_mut()
    }

    pub(crate) fn get_width(&self) -> f64 {
        self.dim_image.width
    }

    pub(crate) fn get_height(&self) -> f64 {
        self.dim_image.height
    }

    pub(crate) fn get_min_x(&self) -> f64 {
        self.min_x.get()
    }

    pub(crate) fn get_min_y(&self) -> f64 {
        self.min_y.get()
    }

    pub(crate) fn get_rectangle_area(&self) -> RectangleArea {
        RectangleArea::new(
            self.get_min_x(),
            self.get_min_y(),
            self.get_min_x() + self.get_width(),
            self.get_min_y() + self.get_height(),
        )
    }

    pub(crate) fn reset_move(&self) {
        self.min_x.set(0.0);
        self.min_y.set(0.0);
    }

    pub(crate) fn move_delta(&self, delta_x: f64, delta_y: f64) {
        self.min_x.set(self.min_x.get() + delta_x);
        self.min_y.set(self.min_y.get() + delta_y);
    }

    /// How far a link ending at `position`, in drawing coordinates, moves to reach the drawn outline.
    #[expect(dead_code, reason = "read by notes drawn around their link")]
    pub(crate) fn get_magnetic_border_force_at(
        &self,
        string_bounder: &dyn StringBounder,
        position: XPoint2D,
    ) -> UTranslate {
        self.image.magnetic_border_force_at(
            string_bounder,
            position.move_by(-self.min_x.get(), -self.min_y.get()),
        )
    }
}
