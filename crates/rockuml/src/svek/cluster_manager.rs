//! Which cluster the nodes being added go in, while groups are opened and closed (PlantUML's
//! `ClusterManager`).
use super::{Bibliotekon, ClusterHeader, ClusterId, IEntityImage};
use crate::abel::{Entity, EntityId};
use crate::klimt::font::StringBounder;

pub(crate) struct ClusterManager {
    current: ClusterId,
}

impl Default for ClusterManager {
    fn default() -> Self {
        Self {
            current: ClusterId::ROOT,
        }
    }
}

impl ClusterManager {
    pub(crate) fn add_node(
        &self,
        bibliotekon: &mut Bibliotekon,
        ent: &Entity,
        image: Box<dyn IEntityImage>,
        string_bounder: &dyn StringBounder,
    ) {
        bibliotekon.create_node(ent, image, self.current, string_bounder);
    }

    pub(crate) fn open_cluster(
        &mut self,
        bibliotekon: &mut Bibliotekon,
        g: EntityId,
        cluster_header: ClusterHeader,
    ) {
        self.current = bibliotekon.create_cluster(self.current, g, cluster_header);
    }

    /// # Panics
    ///
    /// At the root cluster.
    pub(crate) fn close_cluster(&mut self, bibliotekon: &Bibliotekon) {
        self.current = bibliotekon
            .cluster(self.current)
            .get_parent_cluster()
            .expect("a cluster to close");
    }
}
