//! Lays the commit graph out with Smetana, top to bottom, and draws its boxes and dashed arrows (PlantUML's
//! `SmetanaForGit`, `MagicBox` and `GitCurve`).

use smetana::{Drawing, Edge, Node, Point};

use super::gnode::{GNode, GNodeBuilder};
use crate::color::HColor;
use crate::creole::{CreoleMode, Display};
use crate::diagram::json::{Mirror, draw_arrow};
use crate::java::double_to_string;
use crate::klimt::font::{FontConfiguration, StringBounder, UFont, UFontFace};
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::shape::{URectangle, USegment, UShape};
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::skin::component::TextBlockEmpty;
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

fn get_style(skin_param: &SkinParam) -> Style {
    StyleSignature::of(&[SName::Root, SName::Element, SName::GitDiagram])
        .get_merged_style(&skin_param.current_style_builder())
}

pub(super) struct SmetanaForGit<'a> {
    skin_param: &'a SkinParam,
    /// Each node's Smetana node, in the graph's order.
    nodes: Vec<(&'a GNode, Node)>,
    edges: Vec<Edge>,
    drawing: Drawing,
    y_mirror: Mirror,
}

impl<'a> SmetanaForGit<'a> {
    /// The graph laid out, its boxes measured with `string_bounder` (`initGraph`).
    pub(super) fn new(
        skin_param: &'a SkinParam,
        graph: &'a GNodeBuilder,
        string_bounder: &dyn StringBounder,
    ) -> Self {
        let mut g = crate::sdot::new_graph();
        let root = g.root();
        g.set(root, "ranksep", "0.35");
        let mut smetana_nodes = Vec::new();
        for (num, index) in graph.get_all_nodes().enumerate() {
            let dim = MagicBox::new(skin_param, graph.node(index)).get_big_dim(string_bounder);
            let node = g.node(root, &format!("N{num}"));
            g.set(node, "shape", "box");
            g.set(node, "height", &double_to_string(dim.height / 72.0));
            g.set(node, "width", &double_to_string(dim.width / 72.0));
            smetana_nodes.push((index, node));
        }
        let mut edges = Vec::new();
        for &(index, a0) in &smetana_nodes {
            for parent in &graph.node(index).down {
                let Some(&(_, a1)) = smetana_nodes.iter().find(|(other, _)| other == parent) else {
                    continue;
                };
                let edge = g.edge(root, a0, a1);
                g.set(edge, "arrowsize", ".7");
                g.set(edge, "arrowtail", "none");
                g.set(edge, "arrowhead", "normal");
                edges.push(edge);
            }
        }
        g.gv_context();
        #[cfg(test)]
        crate::sdot::recorded_graphs::record(&g);
        let drawing = g
            .layout()
            .expect("Smetana lays out the commits of git graphs");
        let nodes: Vec<_> = smetana_nodes
            .into_iter()
            .map(|(index, node)| (graph.node(index), node))
            .collect();
        let max = nodes
            .iter()
            .map(|&(_, node)| {
                let data = drawing.node(node);
                data.center.y + data.height * 72.0
            })
            .fold(0.0, f64::max);
        Self {
            skin_param,
            nodes,
            edges,
            drawing,
            y_mirror: Mirror::new(max),
        }
    }

    fn get_size(&self, node: Node) -> XDimension2D {
        let data = self.drawing.node(node);
        XDimension2D::new(data.width * 72.0, data.height * 72.0)
    }

    fn get_position(&self, node: Node) -> UTranslate {
        let data = self.drawing.node(node);
        let width = data.width * 72.0;
        let height = data.height * 72.0;
        UTranslate::new(
            data.center.x - width / 2.0,
            self.y_mirror.inv(data.center.y + height / 2.0),
        )
    }
}

impl TextBlock for SmetanaForGit<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;
        for &(_, node) in &self.nodes {
            let pos = self.get_position(node);
            let size = self.get_size(node);
            max_x = max_x.max(pos.dx + size.width);
            max_y = max_y.max(pos.dy + size.height);
        }
        XDimension2D::new(max_x, max_y)
    }

    /// `drawMe`.
    fn draw_u(&self, ug: &UGraphic) {
        let style = get_style(self.skin_param);
        let styled = ug
            .apply(style.value(PName::LineColor).as_color())
            .apply(style.stroke());
        for &(gnode, node) in &self.nodes {
            let pos = self.get_position(node);
            MagicBox::new(self.skin_param, gnode)
                .draw_border(&styled.apply(pos), self.get_size(node));
        }
        let arrow_color = style.value(PName::LineColor).as_color();
        for &edge in &self.edges {
            GitCurve::new(self.drawing.edge(edge).beziers.as_slice(), self.y_mirror)
                .draw_curve(&arrow_color, &styled);
        }
    }
}

/// A commit's box, below the text of its first parentheses for a commit nothing comes after (`MagicBox`).
struct MagicBox<'a> {
    skin_param: &'a SkinParam,
    node: &'a GNode,
    font_color: HColor,
}

impl<'a> MagicBox<'a> {
    fn new(skin_param: &'a SkinParam, node: &'a GNode) -> Self {
        Self {
            skin_param,
            node,
            font_color: get_style(skin_param).value(PName::FontColor).as_color(),
        }
    }

    fn font_configuration(&self, font: UFont) -> FontConfiguration {
        FontConfiguration::new(font, self.font_color.clone(), 8)
            .with_hyperlink_style(self.font_color.clone(), UStroke::SIMPLE)
    }

    fn get_small_block(&self) -> Box<dyn TextBlock + 'a> {
        let font = UFont::monospace(15).with_face(UFontFace::BOLD);
        Box::new(Display::create(self.node.texts.iter().cloned()).create0(
            &self.font_configuration(font),
            HorizontalAlignment::Center,
            self.skin_param,
            0.0,
            CreoleMode::Full,
        ))
    }

    fn get_comment_block(&self) -> Box<dyn TextBlock + 'a> {
        match &self.node.comment {
            Some(comment) if self.node.is_top() => {
                let font = UFont::new("SansSerif", UFontFace::NORMAL, 13);
                Box::new(Display::create([comment.clone()]).create0(
                    &self.font_configuration(font),
                    HorizontalAlignment::Center,
                    self.skin_param,
                    0.0,
                    CreoleMode::Full,
                ))
            }
            _ => Box::new(TextBlockEmpty {
                dimension: XDimension2D::new(0.0, 0.0),
            }),
        }
    }

    fn get_big_dim(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dim_comment = self.get_comment_block().calculate_dimension(string_bounder);
        let dim_small = self.get_small_block().calculate_dimension(string_bounder);
        dim_comment.merge_top_bottom(dim_small).delta(8.0, 2.0)
    }

    fn draw_border(&self, ug: &UGraphic, size_in_dot: XDimension2D) {
        let comment = self.get_comment_block();
        let small = self.get_small_block();
        let string_bounder = ug.string_bounder();
        let move_y = comment.calculate_dimension(string_bounder).height;
        let rect = URectangle::new(size_in_dot.width, size_in_dot.height - move_y).rounded(8.0);
        ug.apply(UStroke::with_thickness(1.5))
            .apply(UTranslate::new(0.0, move_y))
            .draw(&UShape::Rectangle(rect));
        comment.draw_u(ug);
        let small_dim = small.calculate_dimension(string_bounder);
        let delta_width = rect.width - small_dim.width;
        let delta_height = rect.height - small_dim.height;
        small.draw_u(&ug.apply(UTranslate::new(
            delta_width / 2.0,
            move_y + delta_height / 2.0,
        )));
    }
}

/// An edge's dashed route with its arrowhead (`GitCurve`).
struct GitCurve {
    points: Vec<Point>,
    x_mirror: Mirror,
    ep: Option<Point>,
}

impl GitCurve {
    fn new(beziers: &[smetana::Bezier], x_mirror: Mirror) -> Self {
        let [bezier] = beziers else {
            panic!("a git edge has exactly one route, got {}", beziers.len());
        };
        Self {
            points: bezier.points.clone(),
            x_mirror,
            ep: (bezier.ep.x != 0.0 || bezier.ep.y != 0.0).then_some(bezier.ep),
        }
    }

    fn draw_curve(&self, color: &HColor, ug: &UGraphic) {
        let ug = ug.apply(UStroke {
            dash_visible: 2.0,
            dash_space: 2.0,
            thickness: 1.0,
        });
        let point = |index: usize| self.x_mirror.inv_git(self.points[index]);
        let first = point(0);
        let mut segments = vec![USegment::MoveTo(first.x, first.y)];
        for i in (1..self.points.len()).step_by(3) {
            let (pt2, pt3, pt4) = (point(i), point(i + 1), point(i + 2));
            segments.push(USegment::CubicTo {
                ctrl1: (pt2.x, pt2.y),
                ctrl2: (pt3.x, pt3.y),
                end: (pt4.x, pt4.y),
            });
        }
        ug.draw(&UShape::path(segments));
        if let Some(ep) = self.ep {
            let last = point(self.points.len() - 1);
            let true_ep = self.x_mirror.inv_git(ep);
            draw_arrow(last, true_ep, &ug.with_backcolor(color.clone()));
        }
    }
}
