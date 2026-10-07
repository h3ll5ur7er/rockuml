//! Arranging entities nothing links into a square with invisible links, so that they do not lie in one long
//! row (PlantUML's `Magma`, `MagmaList` and `SquareMaker`).

use super::CucaDiagram;
use crate::abel::{EntityId, LinkArg};
use crate::decoration::{LinkDecor, LinkType};

/// How long the side of a square holding `size` elements is.
fn compute_branch(size: usize) -> usize {
    let r = (size as f64).sqrt() as usize;
    if r * r == size { r } else { r + 1 }
}

/// The index of the first element of the square's last row.
fn get_bottom_left(size: usize) -> usize {
    let s = compute_branch(size);
    (size - 1) / s * s
}

/// How two elements of a square are linked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SquareLink {
    /// The first of a row below the first of the row above.
    TopDown,
    /// An element right of the one before it.
    LeftRight,
}

/// The links that lay `data` out row by row, in the order PlantUML makes them.
fn put_in_square<O: Copy>(data: &[O]) -> Vec<(O, O, SquareLink)> {
    let branch = compute_branch(data.len());
    let mut head_branch = 0;
    let mut result = Vec::new();
    for i in 1..data.len() {
        if i - head_branch == branch {
            result.push((data[head_branch], data[i], SquareLink::TopDown));
            head_branch = i;
        } else {
            result.push((data[i - 1], data[i], SquareLink::LeftRight));
        }
    }
    result
}

/// The standalone leaves of one group.
struct Magma {
    standalones: Vec<EntityId>,
}

impl Magma {
    /// The group around the leaves' group; `None` for leaves of the root.
    fn get_container(&self, diagram: &CucaDiagram) -> Option<EntityId> {
        let parent = diagram
            .entity(self.standalones[0])
            .get_parent_container(diagram)?;
        diagram.entity(parent).get_parent_container(diagram)
    }

    fn get_top_left(&self) -> EntityId {
        self.standalones[0]
    }

    fn get_bottom_left(&self) -> EntityId {
        self.standalones[get_bottom_left(self.standalones.len())]
    }

    fn get_top_right(&self) -> EntityId {
        self.standalones[compute_branch(self.standalones.len()) - 1]
    }
}

impl CucaDiagram {
    /// Puts every group's leaves that nothing links, when there are three or more, in a square; then, in each
    /// group holding three or more such squares in its subgroups, puts those squares in a square.
    pub(crate) fn apply_single_strategy(&mut self) {
        let mut magma_list = Vec::new();
        let groups = self.groups_and_root();
        for group in &groups {
            let standalones: Vec<EntityId> = self
                .entity(*group)
                .leafs(self)
                .into_iter()
                .filter(|leaf| self.is_standalone(*leaf))
                .collect();
            if standalones.len() < 3 {
                continue;
            }
            for (from, to, how) in put_in_square(&standalones) {
                self.add_invisible_link(from, to, how);
            }
            magma_list.push(Magma { standalones });
        }
        for group in &groups {
            let magmas: Vec<&Magma> = magma_list
                .iter()
                .filter(|magma| magma.get_container(self) == Some(*group))
                .collect();
            if magmas.len() < 3 {
                continue;
            }
            for (from, to, how) in put_in_square(&magmas) {
                match how {
                    SquareLink::TopDown => {
                        self.add_invisible_link(from.get_bottom_left(), to.get_top_left(), how);
                    }
                    SquareLink::LeftRight => {
                        self.add_invisible_link(from.get_top_right(), to.get_top_left(), how);
                    }
                }
            }
        }
    }

    fn add_invisible_link(&mut self, from: EntityId, to: EntityId, how: SquareLink) {
        let length = match how {
            SquareLink::TopDown => 2,
            SquareLink::LeftRight => 1,
        };
        let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).get_invisible();
        let link = self.new_link(None, from, to, link_type, LinkArg::no_display(length));
        self.add_link(link);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_are_as_small_as_they_can_be() {
        assert_eq!(compute_branch(4), 2);
        assert_eq!(compute_branch(5), 3);
        assert_eq!(compute_branch(9), 3);
        assert_eq!(get_bottom_left(5), 3);
        assert_eq!(get_bottom_left(9), 6);
        assert_eq!(get_bottom_left(3), 2);
    }

    #[test]
    fn rows_link_across_and_their_heads_link_down() {
        use SquareLink::{LeftRight, TopDown};
        assert_eq!(
            put_in_square(&[0, 1, 2, 3, 4]),
            [
                (0, 1, LeftRight),
                (1, 2, LeftRight),
                (0, 3, TopDown),
                (3, 4, LeftRight)
            ]
        );
    }
}
