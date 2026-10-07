//! `gen/lib/label`: placement of external labels (`xlabels.c`), which dot uses for head and tail labels, and the
//! R-tree it finds overlaps with (`index.c`, `node.c`, `split_q.c`, `rectangle.c`).
//!
//! The objects and labels are arrays the caller owns; an object refers to its label by index, and an R-tree leaf
//! to its object by index, where C has pointers. Smetana compares those pointers only for identity.

#![allow(non_camel_case_types, non_snake_case)]

mod index;
mod node;
mod rectangle;
mod split_q;
mod xlabels;

pub use index::{RTreeInsert, RTreeOpen, RTreeSearch};
pub use xlabels::{hd_hil_s_from_xy, label_params_t, object_t, placeLabels, xlabel_t};

/// `NODECARD`: the branches per R-tree node.
const NODECARD: i32 = 64;

/// `Rect_t`: an integer rectangle, `boundary` = (low x, low y, high x, high y).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect_t {
    pub boundary: [i32; 4],
}

/// An R-tree node of an [`RTree`] (`Node_t*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeRef(usize);

/// What a branch points to (`Node_t___or_object_t`): a child node, or in a leaf the caller's data (an index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Child {
    Node(NodeRef),
    Data(usize),
}

/// `Branch_t`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Branch_t {
    pub rect: Rect_t,
    pub child: Option<Child>,
}

/// `Node_t`: a leaf at level 0, an inner node above.
#[derive(Clone, Copy, Debug)]
struct Node_t {
    count: i32,
    level: i32,
    branch: [Branch_t; NODECARD as usize],
}

impl Default for Node_t {
    fn default() -> Self {
        Self {
            count: 0,
            level: 0,
            branch: [Branch_t::default(); NODECARD as usize],
        }
    }
}

/// `PartitionVars`: the split's assignment of branches to two groups.
#[derive(Clone, Copy, Debug)]
struct PartitionVars {
    partition: [i32; NODECARD as usize + 1],
    taken: [i32; NODECARD as usize + 1],
    count: [i32; 2],
    cover: [Rect_t; 2],
    area: [i32; 2],
}

/// `SplitQ_t`: the split's scratch space.
#[derive(Clone, Copy, Debug)]
struct SplitQ_t {
    BranchBuf: [Branch_t; NODECARD as usize + 1],
    CoverSplit: Rect_t,
    Partitions: [PartitionVars; 1],
}

impl Default for SplitQ_t {
    fn default() -> Self {
        Self {
            BranchBuf: [Branch_t::default(); NODECARD as usize + 1],
            CoverSplit: Rect_t::default(),
            Partitions: [PartitionVars {
                partition: [0; NODECARD as usize + 1],
                taken: [0; NODECARD as usize + 1],
                count: [0; 2],
                cover: [Rect_t::default(); 2],
                area: [0; 2],
            }],
        }
    }
}

/// `RTree`, owning its nodes. Smetana's statistics counters are left out: nothing reads them.
#[derive(Debug, Default)]
pub struct RTree {
    nodes: Vec<Node_t>,
    pub root: NodeRef,
    split: SplitQ_t,
    /// Never set, so 0: a split may leave a group with a single branch.
    MinFill: i32,
}

/// `LeafList_t`: the leaf branches a search found, in the order of C's linked list.
pub(crate) type LeafList_t = Vec<Branch_t>;
