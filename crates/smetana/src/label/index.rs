//! `index.c`: the R-tree's operations, as far as xlabels uses them (open, insert, search).

use std::cmp::Ordering;

use super::node::{AddBranch, NodeCover, PickBranch, RTreeNewNode};
use super::rectangle::{CombineRect, Overlap};
use super::{Branch_t, Child, LeafList_t, NodeRef, RTree, Rect_t};

/// `RTreeOpen`: an empty tree.
pub fn RTreeOpen() -> RTree {
    let mut rtp = RTree::default();
    rtp.root = RTreeNewIndex(&mut rtp);
    rtp
}

/// `RTreeNewIndex`: an empty leaf.
fn RTreeNewIndex(rtp: &mut RTree) -> NodeRef {
    let x = RTreeNewNode(rtp);
    rtp.nodes[x.0].level = 0;
    x
}

/// `RTreeSearch`: the leaf branches below `n` whose rectangles overlap `r`. A leaf contributes its matches last
/// slot first (C prepends them to its list); subtrees follow each other in slot order.
pub fn RTreeSearch(rtp: &RTree, n: NodeRef, r: &Rect_t) -> LeafList_t {
    let node = &rtp.nodes[n.0];
    let mut llp = LeafList_t::new();
    if node.level > 0 {
        for b in &node.branch {
            if let Some(Child::Node(child)) = b.child
                && Overlap(r, &b.rect)
            {
                llp.extend(RTreeSearch(rtp, child, r));
            }
        }
    } else {
        for b in &node.branch {
            if b.child.is_some() && Overlap(r, &b.rect) {
                llp.insert(0, *b);
            }
        }
    }
    llp
}

/// `RTreeInsert`: inserts `data` with rectangle `r` at `level` below the root `n`, growing a new root when the old
/// one splits (then returns 1).
pub fn RTreeInsert(rtp: &mut RTree, r: &Rect_t, data: usize, n: &mut NodeRef, level: i32) -> i32 {
    let mut newnode = NodeRef(0);
    let mut result = 0;
    if RTreeInsert2(rtp, r, data, *n, &mut newnode, level) != 0 {
        let newroot = RTreeNewNode(rtp);
        rtp.nodes[newroot.0].level = rtp.nodes[n.0].level + 1;
        let mut b = Branch_t {
            rect: NodeCover(&rtp.nodes[n.0]),
            child: Some(Child::Node(*n)),
        };
        AddBranch(rtp, &b, newroot, None);
        b.rect = NodeCover(&rtp.nodes[newnode.0]);
        b.child = Some(Child::Node(newnode));
        AddBranch(rtp, &b, newroot, None);
        *n = newroot;
        result = 1;
    }
    result
}

/// `RTreeInsert2`: the recursive part of [`RTreeInsert`]; returns 1 when `n` split, the other half in `new_`.
fn RTreeInsert2(
    rtp: &mut RTree,
    r: &Rect_t,
    data: usize,
    n: NodeRef,
    new_: &mut NodeRef,
    level: i32,
) -> i32 {
    let mut n2 = NodeRef(0);
    let node_level = rtp.nodes[n.0].level;
    match node_level.cmp(&level) {
        Ordering::Greater => {
            let i = PickBranch(r, &rtp.nodes[n.0]);
            let Some(Child::Node(child)) = rtp.nodes[n.0].branch[i].child else {
                panic!("an inner R-tree node holds data");
            };
            if RTreeInsert2(rtp, r, data, child, &mut n2, level) == 0 {
                let branch = &mut rtp.nodes[n.0].branch[i];
                branch.rect = CombineRect(r, &branch.rect);
                0
            } else {
                rtp.nodes[n.0].branch[i].rect = NodeCover(&rtp.nodes[child.0]);
                let b = Branch_t {
                    child: Some(Child::Node(n2)),
                    rect: NodeCover(&rtp.nodes[n2.0]),
                };
                AddBranch(rtp, &b, n, Some(new_))
            }
        }
        Ordering::Equal => {
            let b = Branch_t {
                rect: *r,
                child: Some(Child::Data(data)),
            };
            AddBranch(rtp, &b, n, Some(new_))
        }
        Ordering::Less => unimplemented!("RTreeInsert2 below the leaves"),
    }
}
