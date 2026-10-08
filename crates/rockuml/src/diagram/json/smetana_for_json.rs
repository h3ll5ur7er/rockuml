//! Lays a JSON or YAML document out with Smetana: one record node per object or array, its rows the record's
//! fields, an edge from a row to the table of the container it holds. Smetana lays the graph out top to bottom;
//! it is drawn turned to run left to right (PlantUML's `SmetanaForJson`, `JsonCurve`, `Arrow` and `Mirror`).

use smetana::{Bezier, Drawing, Edge, Graph, Node, Point};

use super::highlighted::Highlighted;
use super::text_block_json::{TextBlockJson, apply_stroke_and_line_color};
use crate::java::double_to_string;
use crate::json::JsonValue;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XPoint2D};
use crate::klimt::shape::{UEllipse, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

struct InternalNode<'a> {
    block: TextBlockJson<'a>,
    node: Node,
}

pub(super) struct SmetanaForJson<'a> {
    skin_param: &'a SkinParam,
    diagram_type: SName,
    nodes: Vec<InternalNode<'a>>,
    edges: Vec<Edge>,
    drawing: Drawing,
}

impl<'a> SmetanaForJson<'a> {
    /// The document's tables laid out, their text measured with `string_bounder`.
    pub(super) fn new(
        skin_param: &'a SkinParam,
        diagram_type: SName,
        root: &'a JsonValue,
        highlighted: &[Highlighted],
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let mut builder = GraphBuilder {
            graph: crate::sdot::new_graph(),
            skin_param,
            diagram_type,
            string_bounder,
            num: 0,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        builder.manage_one_node(root, highlighted);
        let GraphBuilder {
            mut graph,
            nodes,
            edges,
            ..
        } = builder;
        graph.gv_context();
        #[cfg(test)]
        crate::sdot::recorded_graphs::record(&graph);
        Self {
            skin_param,
            diagram_type,
            nodes,
            edges,
            drawing: graph
                .layout()
                .expect("Smetana lays out the records of JSON documents"),
        }
    }

    fn get_style_arrow(&self) -> Style {
        StyleSignature::of(&[SName::Root, SName::Element, self.diagram_type, SName::Arrow])
            .get_merged_style(&self.skin_param.current_style_builder())
    }

    fn get_style_node(&self) -> Style {
        StyleSignature::of(&[SName::Root, SName::Element, self.diagram_type, SName::Node])
            .get_merged_style(&self.skin_param.current_style_builder())
    }

    /// The lowest edge of the nodes, which the turned drawing mirrors y against.
    fn mirror(&self) -> Mirror {
        let max = self
            .nodes
            .iter()
            .map(|node| {
                let data = self.drawing.node(node.node);
                data.center.y + data.height * 72.0 / 2.0
            })
            .fold(0.0, f64::max);
        Mirror { max }
    }

    fn get_position(&self, node: Node, mirror: Mirror) -> UTranslate {
        let data = self.drawing.node(node);
        let width = data.width * 72.0;
        let height = data.height * 72.0;
        // `sym()`: the drawing is turned, x and y swapped.
        UTranslate::new(
            mirror.inv(data.center.y + height / 2.0),
            data.center.x - width / 2.0,
        )
    }

    pub(super) fn draw_me(&self, ug: &UGraphic) {
        let mirror = self.mirror();
        for node in &self.nodes {
            let position = self.get_position(node.node, mirror);
            node.block
                .draw_u(&apply_stroke_and_line_color(&self.get_style_node(), ug).apply(position));
        }
        let style_arrow = self.get_style_arrow();
        let color = style_arrow.value(PName::LineColor).as_color();
        for &edge in &self.edges {
            let curve = JsonCurve::new(self.drawing.edge(edge).beziers.as_slice(), mirror, 13.0);
            let ug_arrow = apply_stroke_and_line_color(&style_arrow, ug);
            curve.draw_curve(&color, &ug_arrow);
            curve.draw_spot(&ug_arrow.with_backcolor(color.clone()));
        }
    }
}

/// Builds the Smetana graph of a document (`manageOneNode`, `createNode`, `createEdge`).
struct GraphBuilder<'a, 's> {
    graph: Graph,
    skin_param: &'a SkinParam,
    diagram_type: SName,
    string_bounder: &'s dyn StringBounder,
    num: usize,
    nodes: Vec<InternalNode<'a>>,
    edges: Vec<Edge>,
}

impl<'a> GraphBuilder<'a, '_> {
    fn manage_one_node(&mut self, current: &'a JsonValue, highlighted: &[Highlighted]) -> Node {
        let block = TextBlockJson::new(self.skin_param, self.diagram_type, current, highlighted);
        let string_bounder = self.string_bounder;
        let node1 = self.create_node(
            &block,
            matches!(current, JsonValue::Array(_)),
            string_bounder,
        );
        let children = block.children();
        let keys = block.keys();
        self.nodes.push(InternalNode { block, node: node1 });
        for (i, (child, key)) in children.into_iter().zip(keys).enumerate() {
            if let Some(child) = child {
                let child_bloc = self.manage_one_node(child, &up_one_level(&key, highlighted));
                let edge = self.create_edge(node1, child_bloc, i);
                self.edges.push(edge);
            }
        }
        node1
    }

    fn create_edge(&mut self, a0: Node, a1: Node, num: usize) -> Edge {
        let root = self.graph.root();
        let edge = self.graph.edge(root, a0, a1);
        self.graph.set(edge, "arrowsize", ".75");
        self.graph.set(edge, "arrowtail", "none");
        self.graph.set(edge, "arrowhead", "normal");
        self.graph.set(edge, "tailport", &format!("P{num}"));
        edge
    }

    /// A record node sized like the table; Smetana lays it out top to bottom, so its width is the table's
    /// height and the other way round.
    fn create_node(
        &mut self,
        block: &TextBlockJson<'_>,
        is_array: bool,
        string_bounder: &dyn StringBounder,
    ) -> Node {
        let dim = block.calculate_dimension(string_bounder);
        let key_column_width = block.get_width_col_a(string_bounder);
        let value_column_width = block.get_width_col_b(string_bounder);
        let line_heights = block.get_all_heights(string_bounder);
        let root = self.graph.root();
        let node = self.graph.node(root, &format!("N{}", self.num));
        self.num += 1;
        self.graph.set(node, "shape", "record");
        self.graph
            .set(node, "height", &double_to_string(dim.width / 72.0));
        self.graph
            .set(node, "width", &double_to_string(dim.height / 72.0));
        let dot_label = if is_array {
            get_dot_label_array(key_column_width - 8.0, &line_heights)
        } else {
            get_dot_label_map(
                key_column_width - 8.0,
                value_column_width - 8.0,
                &line_heights,
            )
        };
        if !line_heights.is_empty() {
            self.graph.set(node, "label", &dot_label);
        }
        node
    }
}

fn up_one_level(key: &str, list: &[Highlighted]) -> Vec<Highlighted> {
    list.iter()
        .filter_map(|highlighted| highlighted.up_one_level(key))
        .collect()
}

/// `_dim_H_W_` sizes a record field without text.
fn dim(height: f64, width: f64) -> String {
    format!(
        "_dim_{}_{}_",
        double_to_string(height),
        double_to_string(width)
    )
}

fn get_dot_label_array(width_a: f64, line_heights: &[f64]) -> String {
    line_heights
        .iter()
        .enumerate()
        .map(|(i, &height)| format!("<P{i}>{}", dim(height, width_a)))
        .collect::<Vec<_>>()
        .join("|")
}

fn get_dot_label_map(width_a: f64, width_b: f64, line_heights: &[f64]) -> String {
    let height: f64 = line_heights.iter().sum();
    let fields = line_heights
        .iter()
        .enumerate()
        .map(|(i, &line_height)| format!("<P{i}>{}", dim(line_height, width_b)))
        .collect::<Vec<_>>()
        .join("|");
    format!("{{{}|{{{fields}}}}}", dim(height, width_a))
}

/// Flips y around the drawing's lowest point (`Mirror`).
#[derive(Clone, Copy)]
struct Mirror {
    max: f64,
}

impl Mirror {
    fn inv(self, v: f64) -> f64 {
        self.max - v
    }

    /// The point in the turned drawing (`invAndXYSwitch`).
    fn inv_and_xy_switch(self, point: Point) -> XPoint2D {
        XPoint2D::new(self.inv(point.y), point.x)
    }
}

/// An edge's route in the turned drawing, starting with a dot on its row (`JsonCurve`).
struct JsonCurve {
    points: Vec<Point>,
    mirror: Mirror,
    very_first_line: f64,
    ep: Option<Point>,
}

impl JsonCurve {
    fn new(beziers: &[Bezier], mirror: Mirror, very_first_line: f64) -> Self {
        let [bezier] = beziers else {
            panic!("a JSON edge has exactly one route, got {}", beziers.len());
        };
        Self {
            points: bezier.points.clone(),
            mirror,
            very_first_line,
            ep: (bezier.ep.x != 0.0 || bezier.ep.y != 0.0).then_some(bezier.ep),
        }
    }

    fn point(&self, index: usize) -> XPoint2D {
        self.mirror.inv_and_xy_switch(self.points[index])
    }

    fn draw_curve(&self, color: &crate::color::HColor, ug: &UGraphic) {
        let very_first = self.get_very_first();
        let first = self.point(0);
        let mut segments = vec![
            USegment::MoveTo(very_first.x, very_first.y),
            USegment::LineTo(first.x, first.y),
        ];
        for i in (1..self.points.len()).step_by(3) {
            let (pt2, pt3, pt4) = (self.point(i), self.point(i + 1), self.point(i + 2));
            segments.push(USegment::CubicTo {
                ctrl1: (pt2.x, pt2.y),
                ctrl2: (pt3.x, pt3.y),
                end: (pt4.x, pt4.y),
            });
        }
        ug.draw(&UShape::path(segments));
        if let Some(ep) = self.ep {
            let last = self.point(self.points.len() - 1);
            let true_ep = self.mirror.inv_and_xy_switch(ep);
            draw_arrow(last, true_ep, &ug.with_backcolor(color.clone()));
        }
    }

    fn draw_spot(&self, ug: &UGraphic) {
        const SIZE: f64 = 3.0;
        let very_first = self.get_very_first();
        ug.translated(very_first.x - SIZE, very_first.y - SIZE)
            .apply(UStroke::with_thickness(1.0))
            .draw(&UShape::Ellipse(UEllipse::new(2.0 * SIZE, 2.0 * SIZE)));
    }

    /// The route's first point, moved back along its first segment to its row.
    fn get_very_first(&self) -> XPoint2D {
        supp(self.point(0), self.point(1), self.very_first_line)
    }
}

fn supp(center: XPoint2D, direction: XPoint2D, len: f64) -> XPoint2D {
    let full = center.distance(direction);
    let dx = (center.x - direction.x) / full;
    let dy = (center.y - direction.y) / full;
    XPoint2D::new(center.x + dx * len, center.y + dy * len)
}

/// The arrowhead from `p1` to its tip `p2` (`Arrow.drawArrow`).
fn draw_arrow(p1: XPoint2D, p2: XPoint2D, ug: &UGraphic) {
    const FACTOR: f64 = 0.4;
    const FACTOR2: f64 = 0.3;
    let ug = ug.apply(UStroke::with_thickness(1.0));
    let dist = p1.distance(p2);
    let alpha = libm::atan2(p2.x - p1.x, p2.y - p1.y);
    let point =
        |alpha: f64, len: f64| XPoint2D::new(p1.x + len * alpha.sin(), p1.y + len * alpha.cos());
    let p3 = point(alpha + std::f64::consts::PI / 2.0, dist * FACTOR);
    let p4 = point(alpha - std::f64::consts::PI / 2.0, dist * FACTOR);
    let p11 = point(alpha, dist * FACTOR2);
    ug.draw(&UShape::path(vec![
        USegment::MoveTo(p4.x, p4.y),
        USegment::LineTo(p11.x, p11.y),
        USegment::LineTo(p3.x, p3.y),
        USegment::LineTo(p2.x, p2.y),
        // PlantUML closes the path too, which neither its SVG nor its debug output shows.
        USegment::LineTo(p4.x, p4.y),
    ]));
}
