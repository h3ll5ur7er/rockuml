#![allow(
    dead_code,
    reason = "the images of the class, description and state families have the other shapes"
)]

/// The outline of a node, which decides where links meet it (PlantUML's `ShapeType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShapeType {
    Rectangle,
    RectanglePort,
    RectangleWithCircleInside,
    RectangleHtmlForPorts,
    RoundRectangle,
    Circle,
    Oval,
    Diamond,
    Octagon,
    Folder,
    Hexagon,
    Port,
}
