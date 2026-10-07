#![cfg_attr(test, allow(dead_code, reason = "drawn by the Smetana bridge"))]

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
