//! `dotgen/acyclic.c`: breaks cycles in the fast graph by reversing back edges.

use crate::cgraph::{aghead, agtail};
use crate::core::Globals;
use crate::core::ids::{EdgeId, GraphId, NodeId};
use crate::dotgen::fastgr::{delete_fast_edge, find_fast_edge, merge_oneway, virtual_edge};

/// `reverse_edge`: replaces `e` with an edge the other way, merged into one that exists already.
pub(crate) fn reverse_edge(zz: &mut Globals, e: EdgeId) {
    delete_fast_edge(zz, e);
    let (tail, head) = (agtail(zz, e), aghead(zz, e));
    if let Some(f) = find_fast_edge(zz, head, tail) {
        merge_oneway(zz, e, f);
    } else {
        virtual_edge(zz, head, tail, Some(e));
    }
}

/// `dfs`: reverses the edges that lead back onto the depth-first search stack. A stack of nodes with their
/// out-edge positions replaces the recursion, which goes as deep as the graph.
fn dfs(zz: &mut Globals, n: NodeId) {
    if zz.nd(n).mark != 0 {
        return;
    }
    let enter = |zz: &mut Globals, n: NodeId| {
        zz.nd_mut(n).mark = 1;
        zz.nd_mut(n).onstack = 1;
        (n, 0)
    };
    let mut stack = vec![enter(zz, n)];
    while let Some((n, i)) = stack.last_mut() {
        let n = *n;
        let Some(e) = zz.nd(n).out.get(&zz.edge_lists, *i) else {
            zz.nd_mut(n).onstack = 0;
            stack.pop();
            continue;
        };
        let w = aghead(zz, e);
        if zz.nd(w).onstack != 0 {
            // The edge leaves the list, so the next one takes its place.
            reverse_edge(zz, e);
        } else {
            *i += 1;
            if zz.nd(w).mark == 0 {
                let child = enter(zz, w);
                stack.push(child);
            }
        }
    }
}

/// `acyclic`.
pub(crate) fn acyclic_(zz: &mut Globals, g: GraphId) {
    for c in 0..zz.gd(g).comp.size {
        let first = zz
            .node_lists
            .get(zz.gd(g).comp.list.expect("components"), c);
        zz.gd_mut(g).nlist = first;
        let mut n = first;
        while let Some(nn) = n {
            zz.nd_mut(nn).mark = 0;
            n = zz.nd(nn).next;
        }
        let mut n = first;
        while let Some(nn) = n {
            dfs(zz, nn);
            n = zz.nd(nn).next;
        }
    }
}
