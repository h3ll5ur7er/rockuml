//! `dotgen/flat.c`: flat (same rank) edges before positioning. Adjacent endpoints are marked, and every labeled
//! flat edge between non-adjacent nodes gets a virtual node for its label in the rank above, which may be a new
//! rank -1.

use crate::cgraph::{aghead, agtail};
use crate::core::Globals;
use crate::core::consts::{FLATORDER, NORMAL, VIRTUAL};
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::core::jmath::max;
use crate::dotgen::dotinit::dot_root;
use crate::dotgen::fastgr::{virtual_edge, virtual_node};
use crate::dotgen::mincross::{rank_node, rank_v, rec_reset_vlists, rec_save_vlists};
use crate::h::rank_t;

/// `make_vn_slot`: a new virtual node at position `pos` of rank `r`, shifting the nodes after it.
fn make_vn_slot(zz: &mut Globals, g: GraphId, r: i32, pos: i32) -> NodeId {
    let n = zz.rank(g, r).n;
    let v = zz.node_lists.REALLOC(n + 2, zz.rank(g, r).v);
    zz.rank_mut(g, r).v = Some(v);
    let mut i = n;
    while i > pos {
        let moved = zz.node_lists.get(v, i - 1);
        zz.node_lists.set(v, i, moved);
        let moved = moved.expect("node in rank");
        zz.nd_mut(moved).order += 1;
        i -= 1;
    }
    let n = virtual_node(zz, g);
    zz.node_lists.set(v, pos, Some(n));
    zz.nd_mut(n).order = pos;
    zz.nd_mut(n).rank = r;
    zz.rank_mut(g, r).n += 1;
    let end = zz.rank(g, r).n;
    zz.node_lists.set(v, end, None);
    zz.node_lists.get(v, pos).expect("new node")
}

/// `findlr`: the orders of `u` and `v`, smaller first.
fn findlr(zz: &Globals, u: NodeId, v: NodeId) -> (i32, i32) {
    let l = zz.nd(u).order;
    let r = zz.nd(v).order;
    if l > r { (r, l) } else { (l, r) }
}

/// Indices into `flat_limits`' bounds: hard and soft, left and right.
const HLB: usize = 0;
const HRB: usize = 1;
const SLB: usize = 2;
const SRB: usize = 3;

/// `setbounds`: narrows the slot for a label between `lpos` and `rpos` by the virtual node `v` of the rank above.
fn setbounds(zz: &Globals, v: NodeId, bounds: &mut [i32; 4], lpos: i32, rpos: i32) {
    if zz.nd(v).node_type != VIRTUAL {
        return;
    }
    let ord = zz.nd(v).order;
    if zz.nd(v).in_.size == 0 {
        // Another flat edge's label node.
        let out = zz.nd(v).out;
        let (l, r) = findlr(
            zz,
            aghead(zz, out.get(&zz.edge_lists, 0).expect("out edge")),
            aghead(zz, out.get(&zz.edge_lists, 1).expect("out edge")),
        );
        // The other flat edge could be to the left or right, span this one (ignored), or cross it.
        if r <= lpos {
            bounds[SLB] = ord;
            bounds[HLB] = ord;
        } else if l >= rpos {
            bounds[SRB] = ord;
            bounds[HRB] = ord;
        } else if !(l < lpos && r > rpos) {
            if l < lpos || (l == lpos && r < rpos) {
                bounds[SLB] = ord;
            }
            if r > rpos || (r == rpos && l > lpos) {
                bounds[SRB] = ord;
            }
        }
    } else {
        // A forward edge's virtual node.
        let mut onleft = false;
        let mut onright = false;
        let out = zz.nd(v).out;
        let mut i = 0;
        while let Some(f) = out.get(&zz.edge_lists, i) {
            i += 1;
            let order = zz.nd(aghead(zz, f)).order;
            if order <= lpos {
                onleft = true;
            } else if order >= rpos {
                onright = true;
            }
        }
        if onleft && !onright {
            bounds[HLB] = ord + 1;
        }
        if onright && !onleft {
            bounds[HRB] = ord - 1;
        }
    }
}

/// `flat_limits`: the position in the rank above for the label node of flat edge `e`.
fn flat_limits(zz: &Globals, g: GraphId, e: EdgeId) -> i32 {
    let r = zz.nd(agtail(zz, e)).rank - 1;
    let rank = rank_v(zz, g, r);
    let mut lnode = 0;
    let mut rnode = zz.rank(g, r).n - 1;
    let mut bounds = [0; 4];
    bounds[HLB] = lnode - 1;
    bounds[SLB] = lnode - 1;
    bounds[HRB] = rnode + 1;
    bounds[SRB] = rnode + 1;
    let (lpos, rpos) = findlr(zz, agtail(zz, e), aghead(zz, e));
    while lnode <= rnode {
        let node = |i| zz.node_lists.get(rank, i).expect("node in rank");
        setbounds(zz, node(lnode), &mut bounds, lpos, rpos);
        if lnode != rnode {
            setbounds(zz, node(rnode), &mut bounds, lpos, rpos);
        }
        lnode += 1;
        rnode -= 1;
        if bounds[HRB] - bounds[HLB] <= 1 {
            break;
        }
    }
    if bounds[HLB] <= bounds[HRB] {
        (bounds[HLB] + bounds[HRB] + 1) / 2
    } else {
        (bounds[SLB] + bounds[SRB] + 1) / 2
    }
}

/// `flat_node`: the virtual node for the label of flat edge `e`, between its ends in the rank above. It is
/// recognised by its `ND_alg`, which points back to `e`.
fn flat_node(zz: &mut Globals, e: EdgeId) {
    let Some(label) = zz.ed(e).label else { return };
    let tail = agtail(zz, e);
    let g = dot_root(zz, tail);
    let r = zz.nd(tail).rank;
    let place = flat_limits(zz, g, e);
    // The bottom of the label box, read before make_vn_slot changes the rank.
    let ypos = if let Some(n) = zz.node_lists.get(rank_v(zz, g, r - 1), 0) {
        (zz.nd(n).coord.y - zz.rank(g, r - 1).ht1) as i32
    } else {
        let n = rank_node(zz, g, r, 0);
        (zz.nd(n).coord.y + zz.rank(g, r).ht2 + f64::from(zz.gd(g).ranksep)) as i32
    };
    let vn = make_vn_slot(zz, g, r - 1, place);
    let mut dimen = zz.textlabels[label].dimen;
    if zz.gd(g).GD_flip() {
        std::mem::swap(&mut dimen.x, &mut dimen.y);
    }
    zz.nd_mut(vn).ht = dimen.y;
    let h2 = (zz.nd(vn).ht / 2.0) as i32;
    zz.nd_mut(vn).rw = dimen.x / 2.0;
    zz.nd_mut(vn).lw = dimen.x / 2.0;
    zz.nd_mut(vn).label = Some(label);
    zz.nd_mut(vn).coord.y = f64::from(ypos + h2);

    let ve = virtual_edge(zz, vn, tail, Some(e));
    zz.ed_mut(ve).tail_port.p.x = -zz.nd(vn).lw;
    zz.ed_mut(ve).head_port.p.x = zz.nd(tail).rw;
    zz.ed_mut(ve).edge_type = FLATORDER;
    let head = aghead(zz, e);
    let ve = virtual_edge(zz, vn, head, Some(e));
    zz.ed_mut(ve).tail_port.p.x = zz.nd(vn).rw;
    zz.ed_mut(ve).head_port.p.x = zz.nd(head).lw;
    zz.ed_mut(ve).edge_type = FLATORDER;

    // Another assumed symmetry of a label node's ht1 and ht2.
    let h2 = f64::from(h2);
    let rank = zz.rank_mut(g, r - 1);
    if rank.ht1 < h2 {
        rank.ht1 = h2;
    }
    if rank.ht2 < h2 {
        rank.ht2 = h2;
    }
    zz.nd_mut(vn).alg = Some(e);
}

/// `abomination`: adds rank -1 for the labels of flat edges on rank 0. `GD_rank(g)` moves one element forward,
/// so that `GD_rank(g)[-1]` exists.
fn abomination(zz: &mut Globals, g: GraphId) {
    assert_eq!(zz.gd(g).minrank, 0, "abomination runs once");
    // 3 = one for the new rank, one for the sentinel, one for off-by-one.
    let size = zz.gd(g).maxrank + 3;
    let rptr = zz.ranks.REALLOC(size, zz.gd(g).rank);
    zz.gd_mut(g).rank = Some(rptr.plus_(1));
    let mut r = zz.gd(g).maxrank;
    while r >= 0 {
        *zz.rank_mut(g, r) = *zz.rank(g, r - 1);
        r -= 1;
    }
    let v = zz.node_lists.ALLOC(2);
    let rank = zz.rank_mut(g, r);
    *rank = rank_t {
        n: 0,
        an: 0,
        v: Some(v),
        av: Some(v),
        flat: None,
        ht1: 1.0,
        ht2: 1.0,
        pht1: 1.0,
        pht2: 1.0,
        ..*rank
    };
    zz.gd_mut(g).minrank -= 1;
}

/// `checkFlatAdjacent`: marks flat edge `e` and its virtual edges adjacent if no real node or label node lies
/// between its ends.
fn checkFlatAdjacent(zz: &mut Globals, e: EdgeId) {
    let tn = agtail(zz, e);
    let hn = aghead(zz, e);
    let (lo, hi) = findlr(zz, tn, hn);
    let root = dot_root(zz, tn);
    let rank = rank_v(zz, root, zz.nd(tn).rank);
    let mut i = lo + 1;
    while i < hi {
        let n = zz.node_lists.get(rank, i).expect("node in rank");
        let info = zz.nd(n);
        if (info.node_type == VIRTUAL && info.label.is_some()) || info.node_type == NORMAL {
            break;
        }
        i += 1;
    }
    if i == hi {
        let mut e = Some(e);
        while let Some(ee) = e {
            zz.ed_mut(ee).adjacent = 1;
            e = zz.ed(ee).to_virt;
        }
    }
}

/// Whether `n` has an edge in `ND_flat_in` or `ND_other` that needs a label node on the rank above (PlantUML's
/// patch: Graphviz overlooks the labeled flat edges in `ND_other`, then adds their label nodes to a rank that
/// does not exist; self loops get no label node).
fn needs_label_rank(zz: &Globals, n: NodeId) -> bool {
    let flat_in = zz.nd(n).flat_in;
    let mut j = 0;
    while let Some(e) = flat_in.get(&zz.edge_lists, j) {
        if zz.ed(e).label.is_some() && zz.ed(e).adjacent == 0 {
            return true;
        }
        j += 1;
    }
    let other = zz.nd(n).other;
    (0..other.size).any(|j| {
        let e = other.get(&zz.edge_lists, j).expect("other edge");
        let (t, h) = (agtail(zz, e), aghead(zz, e));
        t != h
            && zz.nd(t).rank == zz.nd(h).rank
            && zz.ed(e).label.is_some()
            && zz.ed(e).adjacent == 0
    })
}

/// The label width of edge `e` across the ranks.
fn label_width(zz: &Globals, g: GraphId, e: EdgeId) -> f64 {
    let dimen = zz.textlabels[zz.ed(e).label.expect("label")].dimen;
    if zz.gd(g).GD_flip() { dimen.y } else { dimen.x }
}

/// `flat_edges`: marks flat edges whose ends are adjacent, and makes label nodes for the labeled ones that are
/// not (adding rank -1 if needed). Adjacent labeled edges keep their label width in `ED_dist` of the
/// representative edge. Returns whether label nodes were made, so that y coordinates must be set again.
pub(crate) fn flat_edges(zz: &mut Globals, g: GraphId) -> bool {
    let mut reset = false;

    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        let flat_out = zz.nd(nn).flat_out;
        if flat_out.list.is_some() {
            for e in flat_out.edges(&zz.edge_lists) {
                checkFlatAdjacent(zz, e);
            }
        }
        let mut j = 0;
        while j < zz.nd(nn).other.size {
            let e = zz.nd(nn).other.get(&zz.edge_lists, j).expect("other edge");
            if zz.nd(aghead(zz, e)).rank == zz.nd(agtail(zz, e)).rank {
                checkFlatAdjacent(zz, e);
            }
            j += 1;
        }
        n = zz.nd(nn).next;
    }

    if zz.rank(g, 0).flat.is_some() || zz.gd(g).n_cluster > 0 {
        let rank0 = rank_v(zz, g, 0);
        let mut i = 0;
        while let Some(n) = zz.node_lists.get(rank0, i) {
            if needs_label_rank(zz, n) {
                abomination(zz, g);
                break;
            }
            i += 1;
        }
    }

    rec_save_vlists(zz, g);
    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        // If n is the tail of any flat edge, one is in flat_out.
        let flat_out = zz.nd(nn).flat_out;
        if flat_out.list.is_some() {
            for e in flat_out.edges(&zz.edge_lists) {
                if zz.ed(e).label.is_none() {
                    continue;
                }
                if zz.ed(e).adjacent != 0 {
                    zz.ed_mut(e).dist = label_width(zz, g, e);
                } else {
                    reset = true;
                    flat_node(zz, e);
                }
            }
            // Look for other flat edges with labels.
            let mut j = 0;
            while j < zz.nd(nn).other.size {
                let e = zz.nd(nn).other.get(&zz.edge_lists, j).expect("other edge");
                j += 1;
                let (t, h) = (agtail(zz, e), aghead(zz, e));
                if zz.nd(t).rank != zz.nd(h).rank || t == h {
                    continue;
                }
                let mut le = e;
                while let Some(v) = zz.ed(le).to_virt {
                    le = v;
                }
                zz.ed_mut(e).adjacent = zz.ed(le).adjacent;
                if zz.ed(e).label.is_none() {
                    continue;
                }
                if zz.ed(e).adjacent != 0 {
                    let lw = label_width(zz, g, e);
                    zz.ed_mut(le).dist = max(lw, zz.ed(le).dist);
                } else {
                    reset = true;
                    flat_node(zz, e);
                }
            }
        }
        n = zz.nd(nn).next;
    }
    if reset {
        rec_reset_vlists(zz, g);
    }
    reset
}
