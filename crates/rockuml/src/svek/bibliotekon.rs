//! The nodes and clusters of one layout, by entity (PlantUML's `Bibliotekon`, without the Graphviz edges).

use super::{Cluster, ClusterHeader, ClusterId, ColorSequence, IEntityImage, SvekNode};
use crate::abel::{Entity, EntityId};
use crate::klimt::font::StringBounder;

pub(crate) struct Bibliotekon {
    /// The root cluster first, then every other cluster in the order they were opened.
    clusters: Vec<Cluster>,
    /// In the order they were created.
    nodes: Vec<SvekNode>,
    color_sequence: ColorSequence,
}

impl Bibliotekon {
    /// With the root cluster of `root`, the group the layout lays out (`CucaDiagramFileMaker`).
    pub(crate) fn new(root: EntityId) -> Self {
        let mut color_sequence = ColorSequence::default();
        let root_cluster = Cluster::new(None, root, None, &mut color_sequence);
        Self {
            clusters: vec![root_cluster],
            nodes: Vec::new(),
            color_sequence,
        }
    }

    /// A node for `ent` drawn by `image`, laid out in `cluster`.
    pub(crate) fn create_node(
        &mut self,
        ent: &Entity,
        image: Box<dyn IEntityImage>,
        cluster: ClusterId,
        string_bounder: &dyn StringBounder,
    ) {
        let node = SvekNode::new(ent.id(), image, &mut self.color_sequence, string_bounder);
        self.nodes.push(node);
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
        id
    }

    pub(crate) fn cluster(&self, id: ClusterId) -> &Cluster {
        &self.clusters[id.0]
    }

    /// The cluster of a group; the root group has none.
    pub(crate) fn get_cluster(&self, ent: EntityId) -> Option<&Cluster> {
        self.clusters[1..]
            .iter()
            .find(|cluster| cluster.get_group() == ent)
    }

    /// The clusters right inside the cluster of `group`, in the order they were opened.
    pub(crate) fn get_children(&self, group: EntityId) -> impl Iterator<Item = &Cluster> {
        let parent = self
            .clusters
            .iter()
            .position(|cluster| cluster.get_group() == group)
            .map(ClusterId);
        self.clusters
            .iter()
            .filter(move |cluster| parent.is_some() && cluster.get_parent_cluster() == parent)
    }

    pub(crate) fn get_node(&self, ent: EntityId) -> Option<&SvekNode> {
        self.nodes.iter().find(|node| node.get_leaf() == ent)
    }

    pub(crate) fn get_node_mut(&mut self, ent: EntityId) -> Option<&mut SvekNode> {
        self.nodes.iter_mut().find(|node| node.get_leaf() == ent)
    }
}
