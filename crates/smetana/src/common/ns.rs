//! `common/ns.c`: network simplex, which dot uses to assign ranks (y) and, on an auxiliary graph, x coordinates.
//!
//! The graph is the fast graph of `G_ns` (`GD_nlist`, `ND_in`, `ND_out`). Its state lives in [`Globals`]:
//! `Tree_node`/`Tree_edge` (the spanning tree), `S_i` (where `leave_edge` resumes searching, which carries over
//! from one call to the next), and `Enter`/`Low`/`Lim`/`Slack` of the entering-edge search.

use std::cmp::{max, min};

use crate::cgraph::attr::agget;
use crate::cgraph::{aghead, agtail};
use crate::common::utils::{dequeue, enqueue, new_queue};
use crate::core::Globals;
use crate::core::consts::{INT_MAX, NORMAL};
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::core::jutils::atoi;

const SEARCHSIZE: i32 = 30;

/// `LENGTH`: the rank difference an edge spans.
fn LENGTH(zz: &Globals, e: EdgeId) -> i32 {
    zz.nd(aghead(zz, e)).rank - zz.nd(agtail(zz, e)).rank
}

/// `SLACK`: how much longer than its minimum length an edge is.
pub(crate) fn SLACK(zz: &Globals, e: EdgeId) -> i32 {
    LENGTH(zz, e) - zz.ed(e).minlen
}

/// `SEQ`: `a <= b <= c`.
fn SEQ(a: i32, b: i32, c: i32) -> bool {
    a <= b && b <= c
}

/// `TREE_EDGE`: whether an edge is in the spanning tree.
fn TREE_EDGE(zz: &Globals, e: EdgeId) -> bool {
    zz.ed(e).tree_index >= 0
}

fn out_edge(zz: &Globals, n: NodeId, i: i32) -> Option<EdgeId> {
    zz.nd(n).out.get(&zz.edge_lists, i)
}

fn in_edge(zz: &Globals, n: NodeId, i: i32) -> Option<EdgeId> {
    zz.nd(n).in_.get(&zz.edge_lists, i)
}

fn tree_out_edge(zz: &Globals, n: NodeId, i: i32) -> Option<EdgeId> {
    zz.nd(n).tree_out.get(&zz.edge_lists, i)
}

fn tree_in_edge(zz: &Globals, n: NodeId, i: i32) -> Option<EdgeId> {
    zz.nd(n).tree_in.get(&zz.edge_lists, i)
}

fn tree_edge(zz: &Globals, i: i32) -> EdgeId {
    zz.Tree_edge.get(&zz.edge_lists, i).expect("tree edge")
}

fn tree_node(zz: &Globals, i: i32) -> NodeId {
    zz.node_lists
        .get(zz.Tree_node.list.expect("Tree_node"), i)
        .expect("tree node")
}

fn nlist(zz: &Globals) -> Option<NodeId> {
    zz.gd(zz.G_ns.expect("G_ns")).nlist
}

/// `add_tree_edge`.
fn add_tree_edge(zz: &mut Globals, e: EdgeId) {
    assert!(!TREE_EDGE(zz, e), "add_tree_edge: missing tree edge");
    zz.ed_mut(e).tree_index = zz.Tree_edge.size;
    let list = zz.Tree_edge.list.expect("Tree_edge");
    zz.edge_lists.set(list, zz.Tree_edge.size, Some(e));
    zz.Tree_edge.size += 1;

    let nodes = zz.Tree_node.list.expect("Tree_node");
    for n in [agtail(zz, e), aghead(zz, e)] {
        if zz.nd(n).mark == 0 {
            zz.node_lists.set(nodes, zz.Tree_node.size, Some(n));
            zz.Tree_node.size += 1;
        }
    }

    let n = agtail(zz, e);
    zz.nd_mut(n).mark = 1;
    let tree_out = zz.nd(n).tree_out;
    let list = tree_out.list.expect("tree_out");
    zz.edge_lists.set(list, tree_out.size, Some(e));
    zz.edge_lists.set(list, tree_out.size + 1, None);
    zz.nd_mut(n).tree_out.size += 1;
    assert!(
        out_edge(zz, n, tree_out.size).is_some(),
        "add_tree_edge: empty outedge list"
    );

    let n = aghead(zz, e);
    zz.nd_mut(n).mark = 1;
    let tree_in = zz.nd(n).tree_in;
    let list = tree_in.list.expect("tree_in");
    zz.edge_lists.set(list, tree_in.size, Some(e));
    zz.edge_lists.set(list, tree_in.size + 1, None);
    zz.nd_mut(n).tree_in.size += 1;
    assert!(
        in_edge(zz, n, tree_in.size).is_some(),
        "add_tree_edge: empty inedge list"
    );
}

/// Removes `e` from a node's tree list (`tree_out` or `tree_in`), moving the last edge into its slot.
fn remove_from_tree_list(zz: &mut Globals, n: NodeId, e: EdgeId, outgoing: bool) {
    let info = zz.nd_mut(n);
    let l = if outgoing {
        &mut info.tree_out
    } else {
        &mut info.tree_in
    };
    l.size -= 1;
    let (list, i) = (l.list.expect("tree list"), l.size);
    let j = (0..=i)
        .find(|&j| zz.edge_lists.get(list, j) == Some(e))
        .unwrap_or(i + 1);
    let last = zz.edge_lists.get(list, i);
    zz.edge_lists.set(list, j, last);
    zz.edge_lists.set(list, i, None);
}

/// Appends `f` to a node's tree list.
fn append_to_tree_list(zz: &mut Globals, n: NodeId, f: EdgeId, outgoing: bool) {
    let info = zz.nd_mut(n);
    let l = if outgoing {
        &mut info.tree_out
    } else {
        &mut info.tree_in
    };
    let list = l.list.expect("tree list");
    let size = l.size;
    l.size += 1;
    zz.edge_lists.set(list, size, Some(f));
    zz.edge_lists.set(list, size + 1, None);
}

/// `exchange_tree_edges`: replaces tree edge `e` with `f`.
fn exchange_tree_edges(zz: &mut Globals, e: EdgeId, f: EdgeId) {
    zz.ed_mut(f).tree_index = zz.ed(e).tree_index;
    let list = zz.Tree_edge.list.expect("Tree_edge");
    zz.edge_lists.set(list, zz.ed(e).tree_index, Some(f));
    zz.ed_mut(e).tree_index = -1;

    let n = agtail(zz, e);
    remove_from_tree_list(zz, n, e, true);
    let n = aghead(zz, e);
    remove_from_tree_list(zz, n, e, false);
    let n = agtail(zz, f);
    append_to_tree_list(zz, n, f, true);
    let n = aghead(zz, f);
    append_to_tree_list(zz, n, f, false);
}

/// `init_rank`: an initial feasible ranking, in topological order. If some nodes are left (a cycle), Graphviz
/// only prints "trouble in `init_rank`" and goes on with their old ranks.
fn init_rank(zz: &mut Globals) {
    let mut Q = new_queue(zz.N_nodes);
    let mut v = nlist(zz);
    while let Some(vv) = v {
        if zz.nd(vv).priority == 0 {
            enqueue(&mut Q, vv);
        }
        v = zz.nd(vv).next;
    }
    while let Some(v) = dequeue(&mut Q) {
        zz.nd_mut(v).rank = 0;
        let mut i = 0;
        while let Some(e) = in_edge(zz, v, i) {
            zz.nd_mut(v).rank = max(zz.nd(v).rank, zz.nd(agtail(zz, e)).rank + zz.ed(e).minlen);
            i += 1;
        }
        let mut i = 0;
        while let Some(e) = out_edge(zz, v, i) {
            let head = aghead(zz, e);
            zz.nd_mut(head).priority -= 1;
            if zz.nd(head).priority <= 0 {
                enqueue(&mut Q, head);
            }
            i += 1;
        }
    }
}

/// `incident`: the end of `e` in the tree when the other end is not, else `None`.
fn incident(zz: &Globals, e: EdgeId) -> Option<NodeId> {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    match (zz.nd(tail).mark != 0, zz.nd(head).mark != 0) {
        (true, false) => Some(tail),
        (false, true) => Some(head),
        _ => None,
    }
}

/// `leave_edge`: a tree edge with a negative cut value, searching from `S_i` round the tree edge list and
/// taking the most negative of the first `Search_size` found.
fn leave_edge(zz: &mut Globals) -> Option<EdgeId> {
    let mut rv = None;
    let mut cnt = 0;
    let j = zz.S_i;
    while zz.S_i < zz.Tree_edge.size {
        if leave_candidate(zz, &mut rv, &mut cnt) {
            return rv;
        }
        zz.S_i += 1;
    }
    if j > 0 {
        zz.S_i = 0;
        while zz.S_i < j {
            if leave_candidate(zz, &mut rv, &mut cnt) {
                return rv;
            }
            zz.S_i += 1;
        }
    }
    rv
}

/// One step of [`leave_edge`]: considers tree edge `S_i`. Returns true when enough candidates were seen.
fn leave_candidate(zz: &Globals, rv: &mut Option<EdgeId>, cnt: &mut i32) -> bool {
    let f = tree_edge(zz, zz.S_i);
    if zz.ed(f).cutvalue >= 0 {
        return false;
    }
    if rv.is_none_or(|r| zz.ed(r).cutvalue > zz.ed(f).cutvalue) {
        *rv = Some(f);
    }
    *cnt += 1;
    *cnt >= zz.Search_size
}

/// `dfs_enter_outedge`: the non-tree edge of least slack leaving the subtree below `v`.
fn dfs_enter_outedge(zz: &mut Globals, v: NodeId) {
    let mut i = 0;
    while let Some(e) = out_edge(zz, v, i) {
        let head = aghead(zz, e);
        if !TREE_EDGE(zz, e) {
            if !SEQ(zz.Low, zz.nd(head).lim, zz.Lim) {
                let slack = SLACK(zz, e);
                if slack < zz.Slack || zz.Enter.is_none() {
                    zz.Enter = Some(e);
                    zz.Slack = slack;
                }
            }
        } else if zz.nd(head).lim < zz.nd(v).lim {
            dfs_enter_outedge(zz, head);
        }
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = tree_in_edge(zz, v, i)
        && zz.Slack > 0
    {
        let tail = agtail(zz, e);
        if zz.nd(tail).lim < zz.nd(v).lim {
            dfs_enter_outedge(zz, tail);
        }
        i += 1;
    }
}

/// `dfs_enter_inedge`: the non-tree edge of least slack entering the subtree below `v`.
fn dfs_enter_inedge(zz: &mut Globals, v: NodeId) {
    let mut i = 0;
    while let Some(e) = in_edge(zz, v, i) {
        let tail = agtail(zz, e);
        if !TREE_EDGE(zz, e) {
            if !SEQ(zz.Low, zz.nd(tail).lim, zz.Lim) {
                let slack = SLACK(zz, e);
                if slack < zz.Slack || zz.Enter.is_none() {
                    zz.Enter = Some(e);
                    zz.Slack = slack;
                }
            }
        } else if zz.nd(tail).lim < zz.nd(v).lim {
            dfs_enter_inedge(zz, tail);
        }
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = tree_out_edge(zz, v, i)
        && zz.Slack > 0
    {
        let head = aghead(zz, e);
        if zz.nd(head).lim < zz.nd(v).lim {
            dfs_enter_inedge(zz, head);
        }
        i += 1;
    }
}

/// `enter_edge`: the non-tree edge to replace tree edge `e`.
fn enter_edge(zz: &mut Globals, e: EdgeId) -> Option<EdgeId> {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    // v is the down node.
    let (v, outsearch) = if zz.nd(tail).lim < zz.nd(head).lim {
        (tail, false)
    } else {
        (head, true)
    };
    zz.Enter = None;
    zz.Slack = INT_MAX;
    zz.Low = zz.nd(v).low;
    zz.Lim = zz.nd(v).lim;
    if outsearch {
        dfs_enter_outedge(zz, v);
    } else {
        dfs_enter_inedge(zz, v);
    }
    zz.Enter
}

/// `treesearch`: grows the tree from `v` along tight edges; true once it spans all nodes.
fn treesearch(zz: &mut Globals, v: NodeId) -> bool {
    let mut i = 0;
    while let Some(e) = out_edge(zz, v, i) {
        let head = aghead(zz, e);
        if zz.nd(head).mark == 0 && SLACK(zz, e) == 0 {
            add_tree_edge(zz, e);
            if zz.Tree_edge.size == zz.N_nodes - 1 || treesearch(zz, head) {
                return true;
            }
        }
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = in_edge(zz, v, i) {
        let tail = agtail(zz, e);
        if zz.nd(tail).mark == 0 && SLACK(zz, e) == 0 {
            add_tree_edge(zz, e);
            if zz.Tree_edge.size == zz.N_nodes - 1 || treesearch(zz, tail) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// `tight_tree`: a tree of tight edges from the first node that has one. Returns its node count.
fn tight_tree(zz: &mut Globals) -> i32 {
    let mut n = nlist(zz);
    while let Some(nn) = n {
        let info = zz.nd_mut(nn);
        info.mark = 0;
        let (tree_in, tree_out) = (info.tree_in.list, info.tree_out.list);
        info.tree_in.size = 0;
        info.tree_out.size = 0;
        zz.edge_lists.set(tree_in.expect("tree_in"), 0, None);
        zz.edge_lists.set(tree_out.expect("tree_out"), 0, None);
        n = zz.nd(nn).next;
    }
    for i in 0..zz.Tree_edge.size {
        let e = tree_edge(zz, i);
        zz.ed_mut(e).tree_index = -1;
    }
    zz.Tree_node.size = 0;
    zz.Tree_edge.size = 0;
    let mut n = nlist(zz);
    while let Some(nn) = n
        && zz.Tree_edge.size == 0
    {
        treesearch(zz, nn);
        n = zz.nd(nn).next;
    }
    zz.Tree_node.size
}

/// `init_cutvalues`.
fn init_cutvalues(zz: &mut Globals) {
    let first = nlist(zz).expect("nodes");
    dfs_range(zz, first, None, 1);
    dfs_cutval(zz, first, None);
}

/// `feasible_tree`: a spanning tree of tight edges, shifting the tree's ranks to make more edges tight.
/// Returns 1 if the graph is not connected.
fn feasible_tree(zz: &mut Globals) -> i32 {
    if zz.N_nodes <= 1 {
        return 0;
    }
    while tight_tree(zz) < zz.N_nodes {
        let mut e: Option<EdgeId> = None;
        let mut n = nlist(zz);
        while let Some(nn) = n {
            let mut i = 0;
            while let Some(f) = out_edge(zz, nn, i) {
                if !TREE_EDGE(zz, f)
                    && incident(zz, f).is_some()
                    && e.is_none_or(|e| SLACK(zz, f) < SLACK(zz, e))
                {
                    e = Some(f);
                }
                i += 1;
            }
            n = zz.nd(nn).next;
        }
        let Some(e) = e else { return 1 };
        let mut delta = SLACK(zz, e);
        if delta != 0 {
            if incident(zz, e) == Some(aghead(zz, e)) {
                delta = -delta;
            }
            for i in 0..zz.Tree_node.size {
                let v = tree_node(zz, i);
                zz.nd_mut(v).rank += delta;
            }
        }
    }
    init_cutvalues(zz);
    0
}

/// `treeupdate`: adds `cutvalue` to the tree edges on the path from `v` up to the common ancestor with `w`,
/// which it returns.
fn treeupdate(zz: &mut Globals, mut v: NodeId, w: NodeId, cutvalue: i32, dir: bool) -> NodeId {
    while !SEQ(zz.nd(v).low, zz.nd(w).lim, zz.nd(v).lim) {
        let e = zz.nd(v).par.expect("tree parent");
        let (tail, head) = (agtail(zz, e), aghead(zz, e));
        let d = if v == tail { dir } else { !dir };
        if d {
            zz.ed_mut(e).cutvalue += cutvalue;
        } else {
            zz.ed_mut(e).cutvalue -= cutvalue;
        }
        v = if zz.nd(tail).lim > zz.nd(head).lim {
            tail
        } else {
            head
        };
    }
    v
}

/// `rerank`: moves the subtree at `v` (away from its parent edge) up by `delta` ranks.
fn rerank(zz: &mut Globals, v: NodeId, delta: i32) {
    zz.nd_mut(v).rank -= delta;
    let par = zz.nd(v).par;
    let mut i = 0;
    while let Some(e) = tree_out_edge(zz, v, i) {
        if Some(e) != par {
            rerank(zz, aghead(zz, e), delta);
        }
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = tree_in_edge(zz, v, i) {
        if Some(e) != par {
            rerank(zz, agtail(zz, e), delta);
        }
        i += 1;
    }
}

/// `update`: exchanges tree edge `e` for `f`, re-ranking the smaller side and updating cut values.
fn update(zz: &mut Globals, e: EdgeId, f: EdgeId) {
    let delta = SLACK(zz, f);
    // Without e the tree falls in two. Moving one part by f's slack makes f tight; the part moved is a lone
    // leaf if either end is one, else the subtree below e (the end with the smaller lim).
    if delta > 0 {
        let (tail, head) = (agtail(zz, e), aghead(zz, e));
        let s = zz.nd(tail).tree_in.size + zz.nd(tail).tree_out.size;
        if s == 1 {
            rerank(zz, tail, delta);
        } else {
            let s = zz.nd(head).tree_in.size + zz.nd(head).tree_out.size;
            if s == 1 {
                rerank(zz, head, -delta);
            } else if zz.nd(tail).lim < zz.nd(head).lim {
                rerank(zz, tail, delta);
            } else {
                rerank(zz, head, -delta);
            }
        }
    }
    let cutvalue = zz.ed(e).cutvalue;
    let (ftail, fhead) = (agtail(zz, f), aghead(zz, f));
    let lca = treeupdate(zz, ftail, fhead, cutvalue, true);
    assert!(
        treeupdate(zz, fhead, ftail, cutvalue, false) == lca,
        "update: mismatched lca in treeupdates"
    );
    zz.ed_mut(f).cutvalue = -cutvalue;
    zz.ed_mut(e).cutvalue = 0;
    exchange_tree_edges(zz, e, f);
    let (par, low) = (zz.nd(lca).par, zz.nd(lca).low);
    dfs_range(zz, lca, par, low);
}

/// `scan_and_normalize`: shifts ranks so that the least rank of a real node is 0.
fn scan_and_normalize(zz: &mut Globals) {
    zz.Minrank = i32::MAX;
    zz.Maxrank = -i32::MAX;
    let mut n = nlist(zz);
    while let Some(nn) = n {
        if zz.nd(nn).node_type == NORMAL {
            zz.Minrank = min(zz.Minrank, zz.nd(nn).rank);
            zz.Maxrank = max(zz.Maxrank, zz.nd(nn).rank);
        }
        n = zz.nd(nn).next;
    }
    if zz.Minrank != 0 {
        let mut n = nlist(zz);
        while let Some(nn) = n {
            zz.nd_mut(nn).rank -= zz.Minrank;
            n = zz.nd(nn).next;
        }
        zz.Maxrank -= zz.Minrank;
        zz.Minrank = 0;
    }
}

/// `freeTreeList`: frees nothing (Smetana's `free` is a no-op) but clears the marks.
fn freeTreeList(zz: &mut Globals) {
    let mut n = nlist(zz);
    while let Some(nn) = n {
        zz.nd_mut(nn).mark = 0;
        n = zz.nd(nn).next;
    }
}

/// `LR_balance`: centers nodes between equally good positions (x coordinates).
fn LR_balance(zz: &mut Globals) {
    for i in 0..zz.Tree_edge.size {
        let e = tree_edge(zz, i);
        if zz.ed(e).cutvalue == 0 {
            let Some(f) = enter_edge(zz, e) else { continue };
            let delta = SLACK(zz, f);
            if delta <= 1 {
                continue;
            }
            let (tail, head) = (agtail(zz, e), aghead(zz, e));
            if zz.nd(tail).lim < zz.nd(head).lim {
                rerank(zz, tail, delta / 2);
            } else {
                rerank(zz, head, -delta / 2);
            }
        }
    }
    freeTreeList(zz);
}

/// `TB_balance`: moves nodes that are not tight, with equal in and out weights, to less populated ranks.
fn TB_balance(zz: &mut Globals) {
    scan_and_normalize(zz);

    let mut nrank = vec![0; (zz.Maxrank + 1) as usize];
    let mut n = nlist(zz);
    while let Some(nn) = n {
        if zz.nd(nn).node_type == NORMAL {
            nrank[zz.nd(nn).rank as usize] += 1;
        }
        n = zz.nd(nn).next;
    }
    let mut n = nlist(zz);
    while let Some(nn) = n {
        n = zz.nd(nn).next;
        if zz.nd(nn).node_type != NORMAL {
            continue;
        }
        let (mut inweight, mut outweight) = (0, 0);
        let mut low = 0;
        let mut high = zz.Maxrank;
        let mut i = 0;
        while let Some(e) = in_edge(zz, nn, i) {
            inweight += zz.ed(e).weight;
            low = max(low, zz.nd(agtail(zz, e)).rank + zz.ed(e).minlen);
            i += 1;
        }
        let mut i = 0;
        while let Some(e) = out_edge(zz, nn, i) {
            outweight += zz.ed(e).weight;
            high = min(high, zz.nd(aghead(zz, e)).rank - zz.ed(e).minlen);
            i += 1;
        }
        if low < 0 {
            // Virtual nodes can have ranks < 0.
            low = 0;
        }
        if inweight == outweight {
            let mut choice = low;
            for i in low + 1..=high {
                if nrank[i as usize] < nrank[choice as usize] {
                    choice = i;
                }
            }
            nrank[zz.nd(nn).rank as usize] -= 1;
            nrank[choice as usize] += 1;
            zz.nd_mut(nn).rank = choice;
        }
        zz.nd_mut(nn).mark = 0;
    }
}

/// `init_graph`: counts nodes, allocates the tree lists and tells whether the current ranks are feasible.
fn init_graph(zz: &mut Globals, g: GraphId) -> bool {
    zz.G_ns = Some(g);
    zz.N_nodes = 0;
    zz.S_i = 0;
    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        zz.nd_mut(nn).mark = 0;
        zz.N_nodes += 1;
        n = zz.nd(nn).next;
    }

    zz.Tree_node.list = Some(zz.node_lists.REALLOC(zz.N_nodes, zz.Tree_node.list));
    zz.Tree_node.size = 0;
    zz.Tree_edge.list = Some(zz.edge_lists.REALLOC(zz.N_nodes, zz.Tree_edge.list));
    zz.Tree_edge.size = 0;

    let mut feasible = true;
    let mut n = zz.gd(g).nlist;
    while let Some(nn) = n {
        zz.nd_mut(nn).priority = 0;
        let mut i = 0;
        while let Some(e) = in_edge(zz, nn, i) {
            zz.nd_mut(nn).priority += 1;
            zz.ed_mut(e).cutvalue = 0;
            zz.ed_mut(e).tree_index = -1;
            if feasible && LENGTH(zz, e) < zz.ed(e).minlen {
                feasible = false;
            }
            i += 1;
        }
        zz.nd_mut(nn).tree_in.list = Some(zz.edge_lists.ALLOC(i + 1));
        zz.nd_mut(nn).tree_in.size = 0;
        let mut i = 0;
        while out_edge(zz, nn, i).is_some() {
            i += 1;
        }
        zz.nd_mut(nn).tree_out.list = Some(zz.edge_lists.ALLOC(i + 1));
        zz.nd_mut(nn).tree_out.size = 0;
        n = zz.nd(nn).next;
    }
    feasible
}

/// `rank2`: network simplex on the fast graph of `g`, then balancing: 1 for ranks (`TB_balance`), 2 for x
/// coordinates (`LR_balance`). Returns 1 if the graph is not connected.
pub fn rank2(zz: &mut Globals, g: GraphId, balance: i32, maxiter: i32, search_size: i32) -> i32 {
    let feasible = init_graph(zz, g);
    if !feasible {
        init_rank(zz);
    }
    if maxiter <= 0 {
        freeTreeList(zz);
        return 0;
    }

    zz.Search_size = if search_size >= 0 {
        search_size
    } else {
        SEARCHSIZE
    };

    if feasible_tree(zz) != 0 {
        freeTreeList(zz);
        return 1;
    }
    let mut iter = 0;
    while let Some(e) = leave_edge(zz) {
        let f = enter_edge(zz, e).expect("entering edge");
        update(zz, e, f);
        iter += 1;
        if iter >= maxiter {
            break;
        }
    }
    match balance {
        1 => TB_balance(zz),
        2 => LR_balance(zz),
        _ => {
            scan_and_normalize(zz);
            freeTreeList(zz);
        }
    }
    0
}

/// `rank`: [`rank2`] with the graph's `searchsize`.
pub fn rank(zz: &mut Globals, g: GraphId, balance: i32, maxiter: i32) -> i32 {
    let search_size = match agget(zz, g, "searchsize") {
        Some(s) => atoi(zz.agstr(s)),
        None => SEARCHSIZE,
    };
    rank2(zz, g, balance, maxiter, search_size)
}

/// `x_cutval`: the cut value of tree edge `f`, from the cut values below it.
fn x_cutval(zz: &mut Globals, f: EdgeId) {
    // v is the end of f on the side already searched.
    let (tail, head) = (agtail(zz, f), aghead(zz, f));
    let (v, dir) = if zz.nd(tail).par == Some(f) {
        (tail, 1)
    } else {
        (head, -1)
    };
    let mut sum = 0;
    let mut i = 0;
    while let Some(e) = out_edge(zz, v, i) {
        sum += x_val(zz, e, v, dir);
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = in_edge(zz, v, i) {
        sum += x_val(zz, e, v, dir);
        i += 1;
    }
    zz.ed_mut(f).cutvalue = sum;
}

/// `x_val`: the contribution of edge `e` at `v` to its parent edge's cut value.
fn x_val(zz: &Globals, e: EdgeId, v: NodeId, dir: i32) -> i32 {
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    let other = if tail == v { head } else { tail };
    let f;
    let mut rv;
    if SEQ(zz.nd(v).low, zz.nd(other).lim, zz.nd(v).lim) {
        f = false;
        rv = if TREE_EDGE(zz, e) {
            zz.ed(e).cutvalue
        } else {
            0
        };
        rv -= zz.ed(e).weight;
    } else {
        f = true;
        rv = zz.ed(e).weight;
    }
    let mut d = if dir > 0 {
        if head == v { 1 } else { -1 }
    } else if tail == v {
        1
    } else {
        -1
    };
    if f {
        d = -d;
    }
    if d < 0 {
        rv = -rv;
    }
    rv
}

/// `dfs_cutval`: the cut values of the tree below `v`, bottom up.
fn dfs_cutval(zz: &mut Globals, v: NodeId, par: Option<EdgeId>) {
    let mut i = 0;
    while let Some(e) = tree_out_edge(zz, v, i) {
        if Some(e) != par {
            dfs_cutval(zz, aghead(zz, e), Some(e));
        }
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = tree_in_edge(zz, v, i) {
        if Some(e) != par {
            dfs_cutval(zz, agtail(zz, e), Some(e));
        }
        i += 1;
    }
    if let Some(par) = par {
        x_cutval(zz, par);
    }
}

/// `dfs_range`: numbers the tree below `v` in postorder (`lim`), each node also getting the least number below
/// it (`low`). Returns the next number.
fn dfs_range(zz: &mut Globals, v: NodeId, par: Option<EdgeId>, low: i32) -> i32 {
    let mut lim = low;
    zz.nd_mut(v).par = par;
    zz.nd_mut(v).low = low;
    let mut i = 0;
    while let Some(e) = tree_out_edge(zz, v, i) {
        if Some(e) != par {
            lim = dfs_range(zz, aghead(zz, e), Some(e), lim);
        }
        i += 1;
    }
    let mut i = 0;
    while let Some(e) = tree_in_edge(zz, v, i) {
        if Some(e) != par {
            lim = dfs_range(zz, agtail(zz, e), Some(e), lim);
        }
        i += 1;
    }
    zz.nd_mut(v).lim = lim;
    lim + 1
}
