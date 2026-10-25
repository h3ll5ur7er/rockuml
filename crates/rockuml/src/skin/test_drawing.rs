//! What a test sees of a drawing: the shadow of each shape, in drawing order, as the debug listing has it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::HColor;
use crate::klimt::debug::{DebugHeader, StringBounderDebug, UGraphicDebug};
use crate::klimt::geom::XDimension2D;
use crate::klimt::ugraphic::UGraphic;

pub(crate) fn drawn_shadows(draw: impl FnOnce(&UGraphic)) -> Vec<String> {
    let debug = Rc::new(RefCell::new(UGraphicDebug::new("DATE".to_owned())));
    draw(&UGraphic::new(
        debug.clone(),
        Rc::new(StringBounderDebug),
        HColor::WHITE,
    ));
    let document = debug.borrow().document(&DebugHeader {
        dimension: XDimension2D::default(),
        scale_factor: 1.0,
        seed: 0,
        svg_link_target: None,
        hover_path_color_rgb: None,
        preserve_aspect_ratio: "none".to_owned(),
    });
    document
        .lines()
        .filter_map(|line| line.trim().strip_prefix("shadow: "))
        .map(str::to_owned)
        .collect()
}
