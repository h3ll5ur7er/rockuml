//! The nodes and clusters of one layout, by entity (PlantUML's `Bibliotekon`, without the Graphviz edges).

use std::collections::HashMap;

use super::{Cluster, ClusterHeader, ClusterId, ColorSequence, IEntityImage, SvekNode};
use crate::abel::{Entity, EntityId};
use crate::klimt::font::StringBounder;

pub(crate) struct Bibliotekon {
    /// The root cluster first, then every other cluster in the order they were opened.
    clusters: Vec<Cluster>,
    cluster_of_group: HashMap<EntityId, ClusterId>,
    /// In the order their leaves were first given a node.
    nodes: Vec<SvekNode>,
    node_of_leaf: HashMap<EntityId, usize>,
    color_sequence: ColorSequence,
}

impl Bibliotekon {
    /// With the root cluster of `root`, the group the layout lays out (`CucaDiagramFileMaker`).
    pub(crate) fn new(root: EntityId) -> Self {
        let mut color_sequence = ColorSequence::default();
        let root_cluster = Cluster::new(None, root, None, &mut color_sequence);
        Self {
            clusters: vec![root_cluster],
            cluster_of_group: HashMap::from([(root, ClusterId(0))]),
            nodes: Vec::new(),
            node_of_leaf: HashMap::new(),
            color_sequence,
        }
    }

    /// A node for `ent` drawn by `image`, laid out in `cluster`. A leaf given a node again keeps its place among
    /// the nodes with the new one, as in PlantUML's map: an empty package, printed as a group's child, is printed
    /// again as a leaf of its parent.
    pub(crate) fn create_node(
        &mut self,
        ent: &Entity,
        image: Box<dyn IEntityImage>,
        cluster: ClusterId,
        string_bounder: &dyn StringBounder,
    ) {
        let node = SvekNode::new(image, &mut self.color_sequence, string_bounder);
        if let Some(&index) = self.node_of_leaf.get(&ent.id()) {
            self.nodes[index] = node;
        } else {
            self.node_of_leaf.insert(ent.id(), self.nodes.len());
            self.nodes.push(node);
        }
        self.clusters[cluster.0].add_node(ent.id());
    }

    /// A cluster for the group `g` inside `parent`.
    pub(crate) fn create_cluster(
        &mut self,
        parent: ClusterId,
        g: EntityId,
        cluster_header: ClusterHeader,
    ) -> ClusterId {
        let child = Cluster::new(
            Some(parent),
            g,
            Some(cluster_header),
            &mut self.color_sequence,
        );
        let id = ClusterId(self.clusters.len());
        self.clusters.push(child);
        self.cluster_of_group.entry(g).or_insert(id);
        id
    }

    pub(crate) fn cluster(&self, id: ClusterId) -> &Cluster {
        &self.clusters[id.0]
    }

    /// The cluster of a group; the root group has none.
    pub(crate) fn get_cluster(&self, ent: EntityId) -> Option<&Cluster> {
        match self.cluster_of_group.get(&ent) {
            Some(&ClusterId(0)) | None => None,
            Some(&id) => Some(self.cluster(id)),
        }
    }

    /// The clusters right inside the cluster of `group`, in the order they were opened.
    pub(crate) fn get_children(&self, group: EntityId) -> impl Iterator<Item = &Cluster> {
        let parent = self.cluster_of_group.get(&group).copied();
        self.clusters
            .iter()
            .filter(move |cluster| parent.is_some() && cluster.get_parent_cluster() == parent)
    }

    pub(crate) fn get_node(&self, ent: EntityId) -> Option<&SvekNode> {
        Some(&self.nodes[*self.node_of_leaf.get(&ent)?])
    }

    pub(crate) fn get_node_mut(&mut self, ent: EntityId) -> Option<&mut SvekNode> {
        Some(&mut self.nodes[*self.node_of_leaf.get(&ent)?])
    }
}
