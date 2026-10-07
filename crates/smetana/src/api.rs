//! The layout API: build a graph with the calls PlantUML makes, in PlantUML's order, then lay it out.
//!
//! Smetana's results depend on the order of the calls that build the graph (it decides object ids and so the
//! iteration order of every phase), so [`Graph`] offers one method per cgraph call PlantUML's drivers make
//! (`CucaDiagramFileMakerSmetana`, `SmetanaForJson`, `SmetanaForGit`) and makes it right away:
//!
//! | PlantUML | [`Graph`] |
//! |---|---|
//! | `Globals.open()`, `agopen(zz, "g", Agdirected, null)` | [`Graph::new`] |
//! | `agsubg(zz, g, name, true)` | [`Graph::subgraph`] |
//! | `agnode(zz, g, name, true)` | [`Graph::node`] |
//! | `agedge(zz, g, tail, head, null, true)` | [`Graph::edge`] |
//! | `agsafeset(zz, obj, name, value, "")` | [`Graph::set`] |
//! | `agsafeset(zz, obj, name, value, def)` | [`Graph::set_with_default`] |
//! | `agsafeset(.., Macro.createHackInitDimensionFromLabel((int) w, (int) h), "")` | [`Graph::set_label_size`] |
//! | `gvContext(zz)` | [`Graph::gv_context`] |
//! | `gvLayoutJobs(zz, gvc, g)` | [`Graph::layout`] |
//!
//! The [`Drawing`] holds what PlantUML reads back from the records afterwards, for the objects made here.

use std::collections::HashMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::cgraph::attr::{agattr, agsafeset};
use crate::cgraph::edge::agedge;
use crate::cgraph::graph::agopen;
use crate::cgraph::node::agnode;
use crate::cgraph::subg::agsubg;
use crate::cgraph::{AGNODE, Agobj};
use crate::core::Globals;
use crate::core::ids::{EdgeId, GraphId, NodeId, TextlabelId};
use crate::gvc::gvlayout::gvLayoutJobs;
use crate::h::cgraph::Agdirected;
use crate::h::{boxf, pointf};

/// The root graph or a subgraph (a cluster if its name starts with `cluster`) of a [`Graph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Subgraph(usize);

/// A node of a [`Graph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Node(usize);

/// An edge of a [`Graph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Edge(usize);

/// An object attributes can be set on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Object {
    Subgraph(Subgraph),
    Node(Node),
    Edge(Edge),
}

impl From<Subgraph> for Object {
    fn from(g: Subgraph) -> Self {
        Self::Subgraph(g)
    }
}

impl From<Node> for Object {
    fn from(n: Node) -> Self {
        Self::Node(n)
    }
}

impl From<Edge> for Object {
    fn from(e: Edge) -> Self {
        Self::Edge(e)
    }
}

/// Objects in creation order, each listed once: cgraph hands back the existing object when asked for a name
/// twice.
struct Registry<I, H> {
    ids: Vec<I>,
    handles: HashMap<I, H>,
}

impl<I: Copy + Eq + std::hash::Hash, H: Copy> Registry<I, H> {
    fn new() -> Self {
        Self {
            ids: Vec::new(),
            handles: HashMap::new(),
        }
    }

    fn handle(&mut self, id: I, make: fn(usize) -> H) -> H {
        let next = self.ids.len();
        *self.handles.entry(id).or_insert_with(|| {
            self.ids.push(id);
            make(next)
        })
    }
}

/// A graph being built for layout: a Smetana context with its root graph, `"g"` and directed like PlantUML's.
pub struct Graph {
    zz: Globals,
    subgraphs: Registry<GraphId, Subgraph>,
    nodes: Registry<NodeId, Node>,
    edges: Registry<EdgeId, Edge>,
    has_context: bool,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// `Globals.open()` and `agopen(zz, "g", Agdirected, null)`.
    pub fn new() -> Self {
        let mut zz = Globals::open();
        let root = agopen(&mut zz, Some("g"), Agdirected);
        let mut subgraphs = Registry::new();
        subgraphs.handle(root, Subgraph);
        Self {
            zz,
            subgraphs,
            nodes: Registry::new(),
            edges: Registry::new(),
            has_context: false,
        }
    }

    /// The root graph.
    pub fn root(&self) -> Subgraph {
        Subgraph(0)
    }

    /// `agsubg(zz, parent, name, true)`: the subgraph of `parent` named `name`, made if new.
    pub fn subgraph(&mut self, parent: Subgraph, name: &str) -> Subgraph {
        let parent = self.subgraphs.ids[parent.0];
        let g = agsubg(&mut self.zz, parent, Some(name), true).expect("agsubg creates");
        self.subgraphs.handle(g, Subgraph)
    }

    /// `agnode(zz, g, name, true)`: the node named `name`, made in `g` if new.
    pub fn node(&mut self, g: Subgraph, name: &str) -> Node {
        let g = self.subgraphs.ids[g.0];
        let n = agnode(&mut self.zz, g, Some(name), true).expect("agnode creates");
        self.nodes.handle(n, Node)
    }

    /// `agedge(zz, g, tail, head, null, true)`: a new edge in `g`.
    pub fn edge(&mut self, g: Subgraph, tail: Node, head: Node) -> Edge {
        let g = self.subgraphs.ids[g.0];
        let (t, h) = (self.nodes.ids[tail.0], self.nodes.ids[head.0]);
        let e = agedge(&mut self.zz, g, t, h, None, true).expect("agedge creates");
        self.edges.handle(e, Edge)
    }

    /// `agsafeset(zz, obj, name, value, "")`: sets an attribute, declaring it with an empty default if new.
    pub fn set(&mut self, obj: impl Into<Object>, name: &str, value: &str) {
        self.set_with_default(obj, name, value, "");
    }

    /// `agsafeset(zz, obj, name, value, default)`: sets an attribute, declaring it with `default` if new (PlantUML
    /// declares `rankdir` with default `LR`).
    pub fn set_with_default(
        &mut self,
        obj: impl Into<Object>,
        name: &str,
        value: &str,
        default: &str,
    ) {
        let obj = self.agobj(obj.into());
        agsafeset(&mut self.zz, obj, name, value, default);
    }

    /// Sets a label attribute to PlantUML's `_dim_W_H_` text, which makes Smetana size the label `w` × `h` points
    /// (truncated to integers like PlantUML's `(int)` casts) instead of measuring text.
    pub fn set_label_size(&mut self, obj: impl Into<Object>, name: &str, width: f64, height: f64) {
        let value = format!("_dim_{}_{}_", width as i32, height as i32);
        self.set(obj, name, &value);
    }

    /// `gvContext(zz)`: declares the default node label. PlantUML calls it once the graph is built, and may set
    /// attributes between it and the layout; [`Graph::layout`] calls it if it has not been called.
    pub fn gv_context(&mut self) {
        assert!(!self.has_context, "gvContext called twice");
        agattr(&mut self.zz, None, AGNODE, "label", Some("\\N"));
        self.has_context = true;
    }

    /// `gvLayoutJobs(zz, gvc, g)`: lays the graph out with dot.
    ///
    /// Smetana throws on input it does not support (attributes PlantUML never sets, graphs that trip its
    /// unported paths); the port panics there, and this returns the panic's message as an error. Catching needs
    /// unwinding: where panics abort, as on `wasm32-unknown-unknown`, such input aborts instead. PlantUML's own
    /// call sequences do not reach these paths. The default panic hook still reports the panic on stderr.
    pub fn layout(mut self) -> Result<Drawing, LayoutError> {
        if !self.has_context {
            self.gv_context();
        }
        let root = self.subgraphs.ids[0];
        let laid_out = catch_unwind(AssertUnwindSafe(|| {
            gvLayoutJobs(&mut self.zz, root);
        }));
        match laid_out {
            Ok(()) => Ok(Drawing::read(&self)),
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "layout failed".to_owned());
                Err(LayoutError { message })
            }
        }
    }

    fn agobj(&self, obj: Object) -> Agobj {
        match obj {
            Object::Subgraph(g) => self.subgraphs.ids[g.0].into(),
            Object::Node(n) => self.nodes.ids[n.0].into(),
            Object::Edge(e) => self.edges.ids[e.0].into(),
        }
    }
}

/// Why Smetana could not lay a graph out: the message of the exception Java would throw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutError {
    message: String,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Smetana layout failed: {}", self.message)
    }
}

impl std::error::Error for LayoutError {}

/// A point in Graphviz's coordinates: points, y up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl From<pointf> for Point {
    fn from(p: pointf) -> Self {
        Self { x: p.x, y: p.y }
    }
}

/// A box by its lower left and upper right corners.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoundingBox {
    pub lower_left: Point,
    pub upper_right: Point,
}

impl From<boxf> for BoundingBox {
    fn from(b: boxf) -> Self {
        Self {
            lower_left: b.LL.into(),
            upper_right: b.UR.into(),
        }
    }
}

/// A placed label (`textlabel_t`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label {
    /// The centre.
    pub pos: Point,
    /// Width and height (`dimen`).
    pub size: Point,
    /// Whether dot placed it (`set`).
    pub placed: bool,
}

/// A node's place (`ND_coord`, `ND_width`, `ND_height`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeLayout {
    pub center: Point,
    /// In inches, as Smetana keeps it; PlantUML multiplies by 72.
    pub width: f64,
    /// In inches, as Smetana keeps it; PlantUML multiplies by 72.
    pub height: f64,
}

/// One piece of an edge's route (`bezier`): cubic bezier control points, and the arrow tips Smetana clipped off
/// (`sflag`/`sp` at the start, `eflag`/`ep` at the end).
#[derive(Clone, Debug, PartialEq)]
pub struct Bezier {
    pub points: Vec<Point>,
    pub sflag: i32,
    pub eflag: i32,
    pub sp: Point,
    pub ep: Point,
}

/// An edge's route (`ED_spl`, empty if it has none) and labels.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeLayout {
    pub beziers: Vec<Bezier>,
    pub label: Option<Label>,
    pub head_label: Option<Label>,
    pub tail_label: Option<Label>,
}

/// A graph's or cluster's box (`GD_bb`) and label.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubgraphLayout {
    pub bb: BoundingBox,
    pub label: Option<Label>,
}

/// The result of a layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    subgraphs: Vec<SubgraphLayout>,
    nodes: Vec<NodeLayout>,
    edges: Vec<EdgeLayout>,
}

impl Drawing {
    fn read(graph: &Graph) -> Self {
        let zz = &graph.zz;
        let label = |l: Option<TextlabelId>| {
            l.map(|l| {
                let l = &zz.textlabels[l];
                Label {
                    pos: l.pos.into(),
                    size: l.dimen.into(),
                    placed: l.set != 0,
                }
            })
        };
        let subgraphs = graph
            .subgraphs
            .ids
            .iter()
            .map(|&g| {
                let info = zz.gd(g);
                SubgraphLayout {
                    bb: info.bb.into(),
                    label: label(info.label),
                }
            })
            .collect();
        let nodes = graph
            .nodes
            .ids
            .iter()
            .map(|&n| {
                let info = zz.nd(n);
                NodeLayout {
                    center: info.coord.into(),
                    width: info.width,
                    height: info.height,
                }
            })
            .collect();
        let edges = graph
            .edges
            .ids
            .iter()
            .map(|&e| {
                let info = zz.ed(e);
                EdgeLayout {
                    beziers: info.spl.map_or_else(Vec::new, |spl| beziers(zz, spl)),
                    label: label(info.label),
                    head_label: label(info.head_label),
                    tail_label: label(info.tail_label),
                }
            })
            .collect();
        Self {
            subgraphs,
            nodes,
            edges,
        }
    }

    /// The bounding box of the whole drawing.
    pub fn bb(&self) -> BoundingBox {
        self.subgraphs[0].bb
    }

    pub fn subgraph(&self, g: Subgraph) -> &SubgraphLayout {
        &self.subgraphs[g.0]
    }

    pub fn node(&self, n: Node) -> &NodeLayout {
        &self.nodes[n.0]
    }

    pub fn edge(&self, e: Edge) -> &EdgeLayout {
        &self.edges[e.0]
    }

    /// The root graph and the subgraphs, in creation order.
    pub fn subgraphs(&self) -> &[SubgraphLayout] {
        &self.subgraphs
    }

    /// The nodes in creation order.
    pub fn nodes(&self) -> &[NodeLayout] {
        &self.nodes
    }

    /// The edges in creation order.
    pub fn edges(&self) -> &[EdgeLayout] {
        &self.edges
    }
}

fn beziers(zz: &Globals, spl: crate::core::ids::SplinesId) -> Vec<Bezier> {
    let spl = zz.splines[spl];
    (0..spl.size)
        .map(|i| {
            let bz = zz.beziers.get(spl.list.expect("beziers"), i);
            Bezier {
                points: (0..bz.size)
                    .map(|k| zz.pointfs.get(bz.list.expect("points"), k).into())
                    .collect(),
                sflag: bz.sflag,
                eflag: bz.eflag,
                sp: bz.sp.into(),
                ep: bz.ep.into(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layout_smetana_rejects_is_an_error() {
        let mut graph = Graph::new();
        let root = graph.root();
        graph.node(root, "a");
        graph.set(root, "ratio", "fill");
        let error = graph.layout().unwrap_err();
        assert!(error.to_string().contains("ratio"), "{error}");
    }

    #[test]
    fn asking_for_a_name_twice_gives_the_same_object() {
        let mut graph = Graph::new();
        let root = graph.root();
        let cluster = graph.subgraph(root, "cluster1");
        let a = graph.node(cluster, "a");
        assert_eq!(graph.node(root, "a"), a);
        assert_eq!(graph.subgraph(root, "cluster1"), cluster);
        let b = graph.node(root, "b");
        let e = graph.edge(root, a, b);
        assert_ne!(graph.edge(root, a, b), e);
        let drawing = graph.layout().unwrap();
        assert_eq!(drawing.nodes().len(), 2);
        assert_eq!(drawing.edges().len(), 2);
        assert_eq!(drawing.subgraphs().len(), 2);
    }
}
