//! `node.c`: R-tree nodes and their branches.

use super::rectangle::{CombineRect, InitRect, RectArea};
use super::split_q::SplitNode;
use super::{Branch_t, NODECARD, Node_t, NodeRef, RTree, Rect_t};

/// `RTreeNewNode`: a fresh node, allocated in the tree.
pub(crate) fn RTreeNewNode(rtp: &mut RTree) -> NodeRef {
    let mut n = Node_t::default();
    InitNode(&mut n);
    rtp.nodes.push(n);
    NodeRef(rtp.nodes.len() - 1)
}

/// `InitNode`.
pub(crate) fn InitNode(n: &mut Node_t) {
    n.count = 0;
    n.level = -1;
    for b in &mut n.branch {
        InitBranch(b);
    }
}

/// `InitBranch`.
fn InitBranch(b: &mut Branch_t) {
    InitRect(&mut b.rect);
    b.child = None;
}

/// `NodeCover`: the rectangle covering all the node's branches.
pub(crate) fn NodeCover(n: &Node_t) -> Rect_t {
    let mut r = Rect_t::default();
    InitRect(&mut r);
    let mut flag = true;
    for b in &n.branch {
        if b.child.is_some() {
            if flag {
                r = b.rect;
                flag = false;
            } else {
                r = CombineRect(&r, &b.rect);
            }
        }
    }
    r
}

/// `PickBranch`: the branch whose rectangle grows least to take `r`; of equal growths the smallest one.
pub(crate) fn PickBranch(r: &Rect_t, n: &Node_t) -> usize {
    let mut flag = true;
    let mut bestIncr = 0;
    let mut bestArea = 0;
    let mut best = 0;
    for (i, b) in n.branch.iter().enumerate() {
        if b.child.is_some() {
            let rr = &b.rect;
            let area = RectArea(rr);
            let rect = CombineRect(r, rr);
            let increase = RectArea(&rect).wrapping_sub(area);
            if increase < bestIncr || flag {
                best = i;
                bestArea = area;
                bestIncr = increase;
                flag = false;
            } else if increase == bestIncr && area < bestArea {
                best = i;
                bestArea = area;
                bestIncr = increase;
            }
        }
    }
    best
}

/// `AddBranch`: puts `b` in the first free slot of `n` and returns 0, or splits a full node into `n` and a new
/// node returned through `new_`, and returns 1.
pub(crate) fn AddBranch(
    rtp: &mut RTree,
    b: &Branch_t,
    n: NodeRef,
    new_: Option<&mut NodeRef>,
) -> i32 {
    let node = &mut rtp.nodes[n.0];
    if node.count < NODECARD {
        if let Some(free) = node.branch.iter_mut().find(|slot| slot.child.is_none()) {
            *free = *b;
            node.count += 1;
        }
        0
    } else {
        let nn = new_.expect("AddBranch splits a node without a place for the new one");
        SplitNode(rtp, n, b, nn);
        1
    }
}
