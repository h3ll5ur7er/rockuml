//! Entity diagrams laid out by Smetana (PlantUML's `sdot` package): the graph a diagram makes, and the
//! drawing of the layout.

mod box_info;
mod cuca_diagram_simplifier_state_smetana;
mod group_maker_state_smetana;
mod padded_entity_image;
mod smetana_edge;
mod text_block_to_entity_image;
mod y_mirror;

use std::cell::OnceCell;

use smetana::{Edge, Graph, Node, Subgraph};

use box_info::BoxInfo;
pub(crate) use cuca_diagram_simplifier_state_smetana::CucaDiagramSimplifierStateSmetana;
use smetana_edge::EdgeTexts;
pub(crate) use smetana_edge::SmetanaEdge;
use text_block_to_entity_image::TextBlockToEntityImage;
use y_mirror::YMirror;

use crate::abel::{EntityId, GroupType, LeafType, Link, LinkId, is_pure_inner_link12};
use crate::creole::{CreoleMode, Display};
use crate::diagram::NotYetPorted;
use crate::diagram::cuca::CucaDiagram;
use crate::java;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, MinMax, XDimension2D, XPoint2D};
use crate::klimt::limit_finder::LimitFinder;
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::Rankdir;
use crate::skin::component::TextBlockEmpty;
use crate::style::{SName, Style, StyleSignature};
use crate::svek::{
    Bibliotekon, ClusterHeader, ClusterManager, IEntityImage, LayoutContext,
    create_entity_image_block,
};

/// Lays a diagram out with Smetana and draws the result (PlantUML's `CucaDiagramFileMakerSmetana` and its
/// base `CucaDiagramFileMaker`). The maker owns the diagram it lays out: drawing changes it, as PlantUML
/// does, so callers hand it a copy.
pub(crate) struct CucaDiagramFileMakerSmetana {
    diagram: CucaDiagram,
    /// The group laid out: the diagram's root, or a group laid out on its own.
    root: EntityId,
    bibliotekon: Bibliotekon,
    cluster_manager: ClusterManager,
    rankdir: Rankdir,
}

/// The graph made for the layout: Smetana's objects by entity and link, in the order they were made.
#[derive(Default)]
struct SmetanaGraph {
    nodes: Vec<(EntityId, Node)>,
    core_nodes: Vec<(EntityId, Node)>,
    edges: Vec<(LinkId, Edge)>,
    clusters: Vec<(EntityId, Subgraph)>,
}

impl SmetanaGraph {
    fn node_of(&self, entity: EntityId) -> Option<Node> {
        self.nodes
            .iter()
            .find(|(known, _)| *known == entity)
            .map(|(_, node)| *node)
    }

    fn cluster_of(&self, group: EntityId) -> Option<Subgraph> {
        self.clusters
            .iter()
            .find(|(known, _)| *known == group)
            .map(|(_, cluster)| *cluster)
    }
}

impl CucaDiagramFileMakerSmetana {
    /// A maker for the whole diagram.
    pub(crate) fn new(diagram: CucaDiagram) -> Self {
        let root = diagram.get_root_group();
        Self::with_root(diagram, root)
    }

    /// A maker for the group `root` alone, as composite states are laid out.
    pub(crate) fn with_root(diagram: CucaDiagram, root: EntityId) -> Self {
        let rankdir = diagram.skin().get_rankdir();
        Self {
            diagram,
            root,
            bibliotekon: Bibliotekon::new(root),
            cluster_manager: ClusterManager::default(),
            rankdir,
        }
    }

    /// The layout of the diagram, drawn.
    pub(crate) fn get_text_block(
        self,
        string_bounder: &dyn StringBounder,
    ) -> Result<Box<dyn TextBlock>, NotYetPorted> {
        self.layout_and_get_text_block(string_bounder)
    }

    /// The layout of a group laid out on its own, as the image of that group (`getImage`).
    pub(crate) fn get_image(
        self,
        string_bounder: &dyn StringBounder,
    ) -> Result<Box<dyn IEntityImage>, NotYetPorted> {
        let text_block = self.layout_and_get_text_block(string_bounder)?;
        Ok(Box::new(TextBlockToEntityImage::new(text_block)))
    }

    fn is_nested_layout(&self) -> bool {
        self.root != self.diagram.get_root_group()
    }

    /// The groups right inside `parent`; a group laid out on its own leaves its concurrent regions out.
    fn get_children_groups(&self, parent: EntityId) -> Vec<EntityId> {
        let groups = self.diagram.entity(parent).groups(&self.diagram);
        if !self.is_nested_layout() {
            return groups;
        }
        groups
            .into_iter()
            .filter(|g| self.diagram.entity(*g).get_group_type() != GroupType::ConcurrentState)
            .collect()
    }

    /// The links to lay out: those of the diagram, or those inside the group laid out on its own.
    fn get_local_links(&self) -> Vec<LinkId> {
        let links = self.diagram.get_link_ids().iter().copied();
        if !self.is_nested_layout() {
            return links.collect();
        }
        let root = self.diagram.entity(self.root);
        links
            .filter(|link| is_pure_inner_link12(root, self.diagram.link(*link), &self.diagram))
            .collect()
    }

    /// The leaves right inside the group laid out; a group laid out on its own leaves its concurrent regions
    /// out.
    fn get_unpackaged_entities(&self) -> Vec<EntityId> {
        self.diagram
            .leafs()
            .into_iter()
            .filter(|ent| {
                let entity = self.diagram.entity(*ent);
                entity.get_parent_container(&self.diagram) == Some(self.root)
                    && !(self.is_nested_layout()
                        && entity.get_leaf_type() == Some(LeafType::StateConcurrent))
            })
            .collect()
    }

    fn print_all_subgroups(
        &mut self,
        string_bounder: &dyn StringBounder,
        parent: EntityId,
    ) -> Result<(), NotYetPorted> {
        for g in self.get_children_groups(parent) {
            let group = self.diagram.entity(g);
            if group.is_removed(&self.diagram) {
                continue;
            }
            if group.is_empty(&self.diagram) && group.get_group_type() == GroupType::Package {
                self.diagram
                    .entity_mut(g)
                    .mute_to_type(LeafType::EmptyPackage);
                self.print_entity(string_bounder, g)?;
            } else {
                self.print_single_group(string_bounder, g)?;
            }
        }
        Ok(())
    }

    fn print_single_group(
        &mut self,
        string_bounder: &dyn StringBounder,
        g: EntityId,
    ) -> Result<(), NotYetPorted> {
        let group = self.diagram.entity(g);
        if group.get_group_type() == GroupType::ConcurrentState {
            return Ok(());
        }
        let packed = group.is_packed();
        if !packed {
            let cluster_header = ClusterHeader::new(group, &self.diagram, string_bounder);
            self.cluster_manager
                .open_cluster(&mut self.bibliotekon, g, cluster_header);
        }
        let leafs = self.diagram.entity(g).leafs(&self.diagram);
        self.print_entities(string_bounder, &leafs)?;
        self.print_all_subgroups(string_bounder, g)?;
        if !packed {
            self.cluster_manager.close_cluster(&self.bibliotekon);
        }
        Ok(())
    }

    fn print_entities(
        &mut self,
        string_bounder: &dyn StringBounder,
        entities: &[EntityId],
    ) -> Result<(), NotYetPorted> {
        for ent in entities {
            if !self.diagram.entity(*ent).is_removed(&self.diagram) {
                self.print_entity(string_bounder, *ent)?;
            }
        }
        Ok(())
    }

    fn print_entity(
        &mut self,
        string_bounder: &dyn StringBounder,
        ent: EntityId,
    ) -> Result<(), NotYetPorted> {
        let image = match self.diagram.get_svek_image(ent) {
            Some(image) => Box::new(image),
            None => create_entity_image_block(ent, &self.diagram, &self.bibliotekon)?,
        };
        self.cluster_manager.add_node(
            &mut self.bibliotekon,
            self.diagram.entity(ent),
            image,
            string_bounder,
        );
        Ok(())
    }

    /// A note whose single visible link goes to something else than a note draws that link itself.
    fn is_opalisable(&self, entity: EntityId) -> bool {
        let entity = self.diagram.entity(entity);
        if entity.get_leaf_type() != Some(LeafType::Note) {
            return false;
        }
        self.only_one_link(entity.id()).is_some_and(|single| {
            self.diagram
                .entity(single.get_other(entity.id()))
                .get_leaf_type()
                != Some(LeafType::Note)
        })
    }

    fn only_one_link(&self, ent: EntityId) -> Option<&Link> {
        let mut links = self
            .diagram
            .get_links()
            .filter(|link| !link.is_invis() && link.contains(ent));
        let single = links.next()?;
        links.next().is_none().then_some(single)
    }

    fn layout_and_get_text_block(
        mut self,
        string_bounder: &dyn StringBounder,
    ) -> Result<Box<dyn TextBlock>, NotYetPorted> {
        self.print_all_subgroups(string_bounder, self.root)?;
        let unpackaged = self.get_unpackaged_entities();
        self.print_entities(string_bounder, &unpackaged)?;
        for link in self.get_local_links() {
            if self.diagram.link(link).is_removed(&self.diagram) {
                continue;
            }
            let (entity1, entity2) = {
                let link = self.diagram.link(link);
                (link.get_entity1(), link.get_entity2())
            };
            let opale = if self.is_opalisable(entity1) {
                Some((entity1, entity2))
            } else if self.is_opalisable(entity2) {
                Some((entity2, entity1))
            } else {
                None
            };
            if let Some((note, other)) = opale
                && self.bibliotekon.get_node(other).is_some()
            {
                self.bibliotekon
                    .get_node_mut(note)
                    .expect("notes are laid out")
                    .get_image_mut()
                    .set_opale_link(link, other);
                self.diagram.link_mut(link).opale = true;
            }
        }
        self.get_text_block_internal(string_bounder)
    }

    fn get_text_block_internal(
        self,
        string_bounder: &dyn StringBounder,
    ) -> Result<Box<dyn TextBlock>, NotYetPorted> {
        let mut graph = Graph::new();
        let g = graph.root();
        // The gap between top-level clusters comes from the margin of the graph holding them.
        graph.set(g, "margin", "16");
        let mut smetana = SmetanaGraph::default();
        self.export_entities(&mut graph, &mut smetana, g, &self.get_unpackaged_entities());
        self.export_groups(&mut graph, &mut smetana, g, self.root);
        for link in self.get_local_links() {
            if self.diagram.link(link).is_removed(&self.diagram) {
                continue;
            }
            if let Some(e) = self.create_edge(string_bounder, &mut graph, &mut smetana, link) {
                smetana.edges.push((link, e));
            }
        }
        if smetana.nodes.is_empty() && smetana.clusters.is_empty() {
            return Ok(Box::new(TextBlockEmpty::default()));
        }
        graph.gv_context();
        if self.rankdir == Rankdir::LeftToRight {
            graph.set_with_default(g, "rankdir", "LR", "LR");
        }
        let layout = graph
            .layout()
            .map_err(|_| NotYetPorted("graphs Smetana cannot lay out"))?;
        // A group laid out on its own is padded by the image around it.
        let canvas_margin = if self.is_nested_layout() { 0.0 } else { 6.0 };
        Ok(Box::new(Drawing::new(self, smetana, layout, canvas_margin)))
    }

    fn export_entities(
        &self,
        graph: &mut Graph,
        smetana: &mut SmetanaGraph,
        cluster: Subgraph,
        entities: &[EntityId],
    ) {
        for ent in entities {
            if !self.diagram.entity(*ent).is_removed(&self.diagram) {
                self.export_entity(graph, smetana, cluster, *ent);
            }
        }
    }

    fn export_entity(
        &self,
        graph: &mut Graph,
        smetana: &mut SmetanaGraph,
        cluster: Subgraph,
        leaf: EntityId,
    ) {
        let node = self
            .bibliotekon
            .get_node(leaf)
            .expect("every leaf exported was printed");
        let agnode = graph.node(cluster, node.get_uid());
        graph.set(agnode, "shape", "box");
        graph.set(
            agnode,
            "width",
            &java::double_to_string(node.get_width() / 72.0),
        );
        graph.set(
            agnode,
            "height",
            &java::double_to_string(node.get_height() / 72.0),
        );
        smetana.nodes.push((leaf, agnode));
    }

    fn export_groups(
        &self,
        graph: &mut Graph,
        smetana: &mut SmetanaGraph,
        cluster: Subgraph,
        parent: EntityId,
    ) {
        for g in self.get_children_groups(parent) {
            let group = self.diagram.entity(g);
            if group.is_removed(&self.diagram) {
                continue;
            }
            if group.is_empty(&self.diagram) && group.get_group_type() == GroupType::Package {
                self.export_entity(graph, smetana, cluster, g);
            } else {
                self.export_group(graph, smetana, cluster, g);
            }
        }
    }

    fn export_group(
        &self,
        graph: &mut Graph,
        smetana: &mut SmetanaGraph,
        parent: Subgraph,
        group: EntityId,
    ) {
        let entity = self.diagram.entity(group);
        if entity.is_packed() {
            self.export_entities(graph, smetana, parent, &entity.leafs(&self.diagram));
            self.export_groups(graph, smetana, parent, group);
            return;
        }
        let Some(cluster) = self.bibliotekon.get_cluster(group) else {
            return;
        };
        let cluster1 = graph.subgraph(parent, &cluster.get_cluster_id());
        if cluster.is_label(&self.diagram) {
            let width = cluster.get_title_and_attribute_width(&self.diagram);
            // Smetana leaves too little room between a title and what is below it: 8 more.
            let height = cluster.get_title_and_attribute_height() - 5 + 8;
            graph.set_label_size(cluster1, "label", f64::from(width), f64::from(height));
        }
        let cluster_margin = if entity
            .get_usymbol()
            .is_some_and(|symbol| symbol.supp_width_because_of_shape() > 0)
        {
            20
        } else {
            16
        };
        graph.set(cluster1, "margin", &cluster_margin.to_string());
        self.export_entities(graph, smetana, cluster1, &entity.leafs(&self.diagram));
        smetana.clusters.push((group, cluster1));
        self.export_groups(graph, smetana, cluster1, group);
    }

    /// The node links to a group end on, made inside its cluster when first needed.
    fn get_core_from_group(
        &self,
        graph: &mut Graph,
        smetana: &mut SmetanaGraph,
        group: EntityId,
    ) -> Node {
        if let Some((_, core)) = smetana.core_nodes.iter().find(|(known, _)| *known == group) {
            return *core;
        }
        let cluster = smetana
            .cluster_of(group)
            .expect("groups linked to are clusters");
        let uid = self.diagram.entity(group).get_uid();
        let result = graph.node(cluster, &format!("z{uid}"));
        graph.set(result, "shape", "box");
        graph.set(result, "width", "0.1");
        graph.set(result, "height", "0.1");
        smetana.core_nodes.push((group, result));
        result
    }

    fn create_edge(
        &self,
        string_bounder: &dyn StringBounder,
        graph: &mut Graph,
        smetana: &mut SmetanaGraph,
        link_id: LinkId,
    ) -> Option<Edge> {
        let link = self.diagram.link(link_id);
        let mut end = |entity: EntityId| {
            if self.diagram.entity(entity).is_group() {
                Some(self.get_core_from_group(graph, smetana, entity))
            } else {
                smetana.node_of(entity)
            }
        };
        let node1 = end(link.get_entity1())?;
        let node2 = end(link.get_entity2())?;
        let g = graph.root();
        let e = graph.edge(g, node1, node2);
        graph.set(e, "arrowtail", "none");
        graph.set(e, "arrowhead", "none");
        graph.set(e, "minlen", &(link.get_length() - 1).to_string());
        let texts = edge_texts(&self.diagram, string_bounder, link);
        if !is_empty(texts.label.as_ref(), string_bounder) {
            let dim_label = texts.label.calculate_dimension(string_bounder);
            graph.set_label_size(e, "label", dim_label.width, dim_label.height);
        }
        if let Some(tail) = &texts.tail_label {
            let dim_label = tail.calculate_dimension(string_bounder);
            graph.set_label_size(e, "taillabel", dim_label.width, dim_label.height);
        }
        if let Some(head) = &texts.head_label {
            let dim_label = head.calculate_dimension(string_bounder);
            graph.set_label_size(e, "headlabel", dim_label.width, dim_label.height);
        }
        Some(e)
    }
}

/// The label, and at each end the quantifier, or the role when there is none; the roles of ends with a
/// quantifier are drawn beside it.
fn edge_texts(diagram: &CucaDiagram, string_bounder: &dyn StringBounder, link: &Link) -> EdgeTexts {
    let quantifier1 = end_text(diagram, string_bounder, link.get_quantifier1());
    let quantifier2 = end_text(diagram, string_bounder, link.get_quantifier2());
    let role1 = end_text(diagram, string_bounder, link.get_role1());
    let role2 = end_text(diagram, string_bounder, link.get_role2());
    let (tail_label, tail_role) = match quantifier1 {
        Some(quantifier) => (Some(quantifier), role1),
        None => (role1, None),
    };
    let (head_label, head_role) = match quantifier2 {
        Some(quantifier) => (Some(quantifier), role2),
        None => (role2, None),
    };
    EdgeTexts {
        label: get_label(diagram, string_bounder, link),
        tail_label,
        head_label,
        tail_role,
        head_role,
    }
}

fn get_label(
    diagram: &CucaDiagram,
    string_bounder: &dyn StringBounder,
    link: &Link,
) -> Box<dyn TextBlock> {
    let skin = diagram.skin();
    let label_only: Box<dyn TextBlock> = match link.get_label() {
        None => Box::new(TextBlockEmpty::default()),
        Some(label) => {
            let arrow_style = get_arrow_style(diagram, link);
            let style_width = arrow_style.wrap_width();
            let wrap_width = if style_width > 0.0 {
                style_width
            } else {
                skin.max_message_size()
            };
            let block = label.create0(
                &arrow_style.font_configuration(),
                get_message_text_alignment(diagram),
                skin,
                wrap_width,
                CreoleMode::FullButUnderscore,
            );
            Box::new(with_margin(block, 1.0))
        }
    };
    if link.note.is_some() {
        unimplemented!("notes on links are drawn once notes are ported")
    }
    if is_empty(label_only.as_ref(), string_bounder) {
        label_only
    } else {
        Box::new(with_margin(label_only, 1.0))
    }
}

/// The style of the link's arrow, with its stereotype, as the link was declared.
fn get_arrow_style(diagram: &CucaDiagram, link: &Link) -> Style {
    arrow_signature(diagram.get_style_name())
        .get_merged_style_with(link.get_style_builder(), link.stereotype.as_ref())
}

fn get_message_text_alignment(diagram: &CucaDiagram) -> HorizontalAlignment {
    let skin = diagram.skin();
    if diagram.get_style_name() == SName::StateDiagram {
        return skin
            .value("stateMessageAlignment")
            .and_then(|value| HorizontalAlignment::from_name(&value))
            .unwrap_or_else(|| skin.get_default_text_alignment(HorizontalAlignment::Center));
    }
    skin.get_default_text_alignment(HorizontalAlignment::Center)
}

/// A quantifier or role at an end of a link.
fn end_text(
    diagram: &CucaDiagram,
    string_bounder: &dyn StringBounder,
    text: Option<&str>,
) -> Option<Box<dyn TextBlock>> {
    let text = text?;
    let skin = diagram.skin();
    let label_font: FontConfiguration = arrow_signature(diagram.get_style_name())
        .get_merged_style(&skin.current_style_builder())
        .font_configuration();
    let label = Display::with_newlines(text).create0(
        &label_font,
        skin.get_default_text_alignment(HorizontalAlignment::Center),
        skin,
        0.0,
        CreoleMode::Full,
    );
    if is_empty(&label, string_bounder) {
        return Some(Box::new(label));
    }
    Some(Box::new(with_margin(label, 1.0)))
}

fn arrow_signature(style_name: SName) -> StyleSignature {
    StyleSignature::of(&[SName::Root, SName::Element, style_name, SName::Arrow])
}

/// `TextBlockUtils.isEmpty`: takes no room.
fn is_empty(text: &dyn TextBlock, string_bounder: &dyn StringBounder) -> bool {
    let dim = text.calculate_dimension(string_bounder);
    dim.height == 0.0 && dim.width == 0.0
}

/// `TextBlockUtils.withMargin(text, margin, margin)`.
fn with_margin<T: TextBlock>(text: T, margin: f64) -> TextBlockMarged<T> {
    TextBlockMarged::new(text, ClockwiseTopRightBottomLeft::same(margin))
}

/// The laid out diagram (`CucaDiagramFileMakerSmetana.Drawing`).
struct Drawing {
    diagram: CucaDiagram,
    bibliotekon: Bibliotekon,
    smetana: SmetanaGraph,
    layout: smetana::Drawing,
    ymirror: YMirror,
    min_max: MinMax,
    canvas_margin: f64,
    /// What drawing really covers, which can overflow the layout's boxes, as self-links do.
    measured_min_max: OnceCell<MinMax>,
}

impl Drawing {
    fn new(
        maker: CucaDiagramFileMakerSmetana,
        smetana: SmetanaGraph,
        layout: smetana::Drawing,
        canvas_margin: f64,
    ) -> Self {
        let min_max = get_smetana_min_max(&smetana, &layout);
        Self {
            diagram: maker.diagram,
            bibliotekon: maker.bibliotekon,
            smetana,
            layout,
            ymirror: YMirror::new(min_max.max_y() + 6.0),
            min_max,
            canvas_margin,
            measured_min_max: OnceCell::new(),
        }
    }

    fn get_baseline_translate(&self) -> (f64, f64) {
        (
            self.canvas_margin,
            self.canvas_margin - self.min_max.min_y(),
        )
    }

    fn get_baseline_dimension(&self) -> XDimension2D {
        self.min_max
            .dimension()
            .delta(2.0 * self.canvas_margin + 4.0, self.canvas_margin + 16.0)
    }

    fn get_measured_min_max(&self, string_bounder: &dyn StringBounder) -> MinMax {
        *self.measured_min_max.get_or_init(|| {
            let (ug, finder) = LimitFinder::surface(string_bounder.shared(), MinMax::empty());
            let (dx, dy) = self.get_baseline_translate();
            self.draw_content(&ug.translated(dx, dy));
            finder.borrow().min_max()
        })
    }

    /// Draws everything where the layout put it; the caller places `ug` at the drawing's origin.
    fn draw_content(&self, ug: &UGraphic) {
        let smetana_pathes: Vec<(LinkId, SmetanaEdge)> = self
            .smetana
            .edges
            .iter()
            .filter(|(link, _)| !self.diagram.link(*link).is_invis())
            .map(|(link, edge)| {
                let texts =
                    edge_texts(&self.diagram, ug.string_bounder(), self.diagram.link(*link));
                (
                    *link,
                    SmetanaEdge::new(*link, self.layout.edge(*edge).clone(), self.ymirror, texts),
                )
            })
            .collect();
        for (group, cluster) in &self.smetana.clusters {
            self.draw_group(ug, *group, *cluster);
        }
        let context = LayoutContext {
            diagram: &self.diagram,
            bibliotekon: &self.bibliotekon,
            smetana_pathes: &smetana_pathes,
        };
        for (leaf, agnode) in &self.smetana.nodes {
            let corner = self.get_corner(*agnode);
            let node = context.get_node(*leaf);
            node.reset_move();
            node.move_delta(corner.x, corner.y);
            node.get_image()
                .draw_u_in_layout(&ug.translated(corner.x, corner.y), &context);
        }
        for (link, edge) in &smetana_pathes {
            if !self.diagram.link(*link).opale {
                edge.draw_u(ug, &self.diagram, &self.bibliotekon);
            }
        }
    }

    fn get_corner(&self, agnode: Node) -> XPoint2D {
        let data = BoxInfo::from_node(self.layout.node(agnode));
        self.ymirror.get_mirrored(data.get_lower_left())
    }

    fn draw_group(&self, ug: &UGraphic, group: EntityId, cluster: Subgraph) {
        let box_info = BoxInfo::from_graph_info(&self.layout.subgraph(cluster).bb);
        let upper_right = self.ymirror.get_mirrored(box_info.get_upper_right());
        let lower_left = self.ymirror.get_mirrored(box_info.get_lower_left());
        let cluster = self
            .bibliotekon
            .get_cluster(group)
            .expect("clusters drawn have been opened");
        cluster.set_position(upper_right, lower_left);
        cluster.draw_u(ug, &self.diagram);
    }
}

impl TextBlock for Drawing {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let measured = self.get_measured_min_max(string_bounder);
        let baseline = self.get_baseline_dimension();
        let extra_left = (-measured.min_x()).max(0.0);
        let extra_top = (-measured.min_y()).max(0.0);
        let extra_right = (measured.max_x() - baseline.width).max(0.0);
        let extra_bottom = (measured.max_y() - baseline.height).max(0.0);
        XDimension2D::new(
            baseline.width + extra_left + extra_right,
            baseline.height + extra_top + extra_bottom,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let measured = self.get_measured_min_max(ug.string_bounder());
        let extra_left = (-measured.min_x()).max(0.0);
        let extra_top = (-measured.min_y()).max(0.0);
        let (dx, dy) = self.get_baseline_translate();
        self.draw_content(&ug.translated(dx, dy).translated(extra_left, extra_top));
    }
}

/// The box around every node and cluster of the layout.
fn get_smetana_min_max(smetana: &SmetanaGraph, layout: &smetana::Drawing) -> MinMax {
    let nodes = smetana
        .nodes
        .iter()
        .map(|(_, node)| BoxInfo::from_node(layout.node(*node)));
    let clusters = smetana
        .clusters
        .iter()
        .map(|(_, cluster)| BoxInfo::from_graph_info(&layout.subgraph(*cluster).bb));
    nodes.chain(clusters).fold(MinMax::empty(), |result, data| {
        let (upper_right, lower_left) = (data.get_upper_right(), data.get_lower_left());
        result
            .add_point(upper_right.x, upper_right.y)
            .add_point(lower_left.x, lower_left.y)
    })
}
