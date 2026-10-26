//! The commit graph: a node per commit, linked to its parents, with straight runs of commits merged into one
//! node (PlantUML's `GNode` and `GNodeBuilder`).

use super::git_text_area::{Commit, Unreadable};

/// A box of the drawing: one commit, or a straight run of them (`GNode`). Links are indexes in
/// [`GNodeBuilder::arena`].
pub(super) struct GNode {
    up: Vec<usize>,
    pub down: Vec<usize>,
    pub texts: Vec<String>,
    pub comment: Option<String>,
}

impl GNode {
    pub(super) fn is_top(&self) -> bool {
        self.up.is_empty()
    }
}

/// The nodes of the commits, in the order their commits first came (`GNodeBuilder`).
pub(super) struct GNodeBuilder {
    arena: Vec<GNode>,
    all: Vec<usize>,
}

impl GNodeBuilder {
    pub(super) fn new(all_commits: &[Commit<'_>]) -> Result<Self, Unreadable> {
        let mut arena = Vec::new();
        // A `LinkedHashMap` by name: a commit named again replaces the node, in the first one's place.
        let mut tmp: Vec<(&str, usize)> = Vec::new();
        for commit in all_commits {
            let node = arena.len();
            arena.push(GNode {
                up: Vec::new(),
                down: Vec::new(),
                texts: vec![commit.name.clone()],
                comment: commit.comment.clone(),
            });
            match tmp.iter_mut().find(|(name, _)| *name == commit.name) {
                Some(entry) => entry.1 = node,
                None => tmp.push((&commit.name, node)),
            }
        }
        let node_of = |name: &str| {
            tmp.iter()
                .find(|(known, _)| *known == name)
                .map(|&(_, node)| node)
        };
        for commit in all_commits {
            for parent in commit.get_ancestors()? {
                // PlantUML throws on a parent no line names.
                let n2 = node_of(&parent.name).ok_or(Unreadable)?;
                let n1 = node_of(&commit.name).expect("every commit has a node");
                arena[n1].down.push(n2);
                arena[n2].up.push(n1);
            }
        }
        let mut builder = Self {
            all: tmp.iter().map(|&(_, node)| node).collect(),
            arena,
        };
        builder.merge();
        Ok(builder)
    }

    fn merge(&mut self) {
        while let Some(&node) = self
            .all
            .iter()
            .find(|&&node| self.can_eat_the_next_one(node))
        {
            let removed = self.eat_the_next_one(node);
            self.all.retain(|&other| other != removed);
        }
    }

    /// The node has one parent and one child, and so has its child (`canEatTheNextOne`).
    fn can_eat_the_next_one(&self, index: usize) -> bool {
        let node = &self.arena[index];
        if node.up.len() != 1 || node.down.len() != 1 {
            return false;
        }
        let next = &self.arena[node.down[0]];
        next.up.len() == 1 && next.down.len() == 1
    }

    /// Takes the child's commits and links to the child's child (`eatTheNextOne`). A loop of commits named
    /// twice shrinks to a node that is its own child, which then eats itself and so leaves the graph.
    fn eat_the_next_one(&mut self, node: usize) -> usize {
        let removed = self.arena[node].down[0];
        let new_next = self.arena[removed].down[0];
        let texts = self.arena[removed].texts.clone();
        self.arena[node].texts.extend(texts);
        self.arena[node].down[0] = new_next;
        let up = &mut self.arena[new_next].up;
        let at = up
            .iter()
            .position(|&parent| parent == removed)
            .expect("the child's child links back to the child");
        up.remove(at);
        up.push(node);
        removed
    }

    /// The nodes left, in order.
    pub(super) fn get_all_nodes(&self) -> impl Iterator<Item = usize> + '_ {
        self.all.iter().copied()
    }

    pub(super) fn node(&self, node: usize) -> &GNode {
        &self.arena[node]
    }
}

#[cfg(test)]
mod tests {
    use super::super::git_text_area::GitTextArea;
    use super::*;

    /// Each node left as its commits.
    fn nodes(lines: &[&str]) -> Vec<String> {
        let area = GitTextArea::new(lines.iter().copied());
        let builder = GNodeBuilder::new(&area.get_all_commits().unwrap()).unwrap();
        builder
            .get_all_nodes()
            .map(|node| builder.node(node).texts.join(" "))
            .collect()
    }

    #[test]
    fn straight_runs_below_the_top_merge_into_one_node() {
        let lines = ["* f Top", "* e", "* d", "* c", "* b", "* a Root"];
        assert_eq!(nodes(&lines), ["f", "e d c b", "a"]);
    }

    #[test]
    fn forks_end_runs() {
        let lines = ["* d", "| * c", "| * b", "|/", "* a", "* r"];
        assert_eq!(nodes(&lines), ["d", "c", "b", "a", "r"]);
    }

    #[test]
    fn a_name_used_again_takes_the_first_ones_place() {
        let lines = ["* b First", "* a", "", "* b Again"];
        assert_eq!(nodes(&lines), ["b", "a"]);
    }

    #[test]
    fn a_loop_eats_itself_away() {
        assert_eq!(nodes(&["* a", "* a"]), [""; 0]);
        assert_eq!(nodes(&["* b First", "* a", "* b Again"]), [""; 0]);
    }
}
