//! `split_q.c`: Guttman's quadratic split of a full R-tree node.

use super::node::{AddBranch, InitNode, RTreeNewNode};
use super::rectangle::{CombineRect, NullRect, RectArea};
use super::{Branch_t, NODECARD, NodeRef, RTree};

const NODECARD_PLUS_1: usize = NODECARD as usize + 1;

/// `SplitNode`: distributes the branches of the full node `n` and the extra branch `b` over `n` and a new node,
/// returned in `nn`.
pub(crate) fn SplitNode(rtp: &mut RTree, n: NodeRef, b: &Branch_t, nn: &mut NodeRef) {
    let level = rtp.nodes[n.0].level;
    GetBranches(rtp, n, b);
    MethodZero(rtp);
    // As in GetBranches: the area feeds statistics only, but computing it may throw.
    let p = &rtp.split.Partitions[0];
    RectArea(&p.cover[0]);
    RectArea(&p.cover[1]);
    *nn = RTreeNewNode(rtp);
    rtp.nodes[n.0].level = level;
    rtp.nodes[nn.0].level = level;
    LoadNodes(rtp, n, *nn);
}

/// `GetBranches`: moves the branches of `n` and `b` to the split buffer and empties `n`.
fn GetBranches(rtp: &mut RTree, n: NodeRef, b: &Branch_t) {
    let split = &mut rtp.split;
    split.BranchBuf[..NODECARD as usize].copy_from_slice(&rtp.nodes[n.0].branch);
    split.BranchBuf[NODECARD as usize] = *b;
    split.CoverSplit = split.BranchBuf[0].rect;
    for i in 1..NODECARD_PLUS_1 {
        split.CoverSplit = CombineRect(&split.CoverSplit, &split.BranchBuf[i].rect);
    }
    // Only Smetana's statistics use the area, but computing it throws where Java throws.
    RectArea(&split.CoverSplit);
    InitNode(&mut rtp.nodes[n.0]);
}

/// `MethodZero`: picks two seeds, then adds the branch with the strongest preference for one group, one at a time.
fn MethodZero(rtp: &mut RTree) {
    InitPVars(rtp);
    PickSeeds(rtp);
    let fill_limit = NODECARD + 1 - rtp.MinFill;
    let mut chosen = 0;
    let mut betterGroup = 0;
    loop {
        let p = &rtp.split.Partitions[0];
        if !(p.count[0] + p.count[1] < NODECARD + 1
            && p.count[0] < fill_limit
            && p.count[1] < fill_limit)
        {
            break;
        }
        let mut biggestDiff = -1;
        for i in 0..NODECARD_PLUS_1 {
            if p.taken[i] == 0 {
                let r = &rtp.split.BranchBuf[i].rect;
                let rect = CombineRect(r, &p.cover[0]);
                let growth0 = RectArea(&rect).wrapping_sub(p.area[0]);
                let rect = CombineRect(r, &p.cover[1]);
                let growth1 = RectArea(&rect).wrapping_sub(p.area[1]);
                let mut diff = growth1.wrapping_sub(growth0);
                let group;
                if diff >= 0 {
                    group = 0;
                } else {
                    group = 1;
                    diff = diff.wrapping_neg();
                }
                if diff > biggestDiff {
                    biggestDiff = diff;
                    chosen = i;
                    betterGroup = group;
                } else if diff == biggestDiff && p.count[group] < p.count[betterGroup] {
                    chosen = i;
                    betterGroup = group;
                }
            }
        }
        Classify(rtp, chosen, betterGroup);
    }
    let p = &rtp.split.Partitions[0];
    if p.count[0] + p.count[1] < NODECARD + 1 {
        unimplemented!("MethodZero with a minimum fill");
    }
}

/// `PickSeeds`: the two branches that would waste the most area together. When no pair wastes any (all
/// rectangles alike), both seeds are branch 0, which ends up in group 1 only and leaves one branch unassigned:
/// [`LoadNodes`] then drops it, as Smetana does.
fn PickSeeds(rtp: &mut RTree) {
    let split = &rtp.split;
    let mut area = [0; NODECARD_PLUS_1];
    for (i, a) in area.iter_mut().enumerate() {
        *a = RectArea(&split.BranchBuf[i].rect);
    }
    let mut worst = 0;
    let mut seed0 = 0;
    let mut seed1 = 0;
    for i in 0..NODECARD as usize {
        for j in i + 1..NODECARD_PLUS_1 {
            let rect = CombineRect(&split.BranchBuf[i].rect, &split.BranchBuf[j].rect);
            let waste = RectArea(&rect).wrapping_sub(area[i]).wrapping_sub(area[j]);
            if waste > worst {
                worst = waste;
                seed0 = i;
                seed1 = j;
            }
        }
    }
    Classify(rtp, seed0, 0);
    Classify(rtp, seed1, 1);
}

/// `Classify`: puts branch `i` in `group`.
fn Classify(rtp: &mut RTree, i: usize, group: usize) {
    let split = &mut rtp.split;
    let p = &mut split.Partitions[0];
    p.partition[i] = group as i32;
    p.taken[i] = 1;
    if p.count[group] == 0 {
        p.cover[group] = split.BranchBuf[i].rect;
    } else {
        p.cover[group] = CombineRect(&split.BranchBuf[i].rect, &p.cover[group]);
    }
    p.area[group] = RectArea(&p.cover[group]);
    p.count[group] += 1;
}

/// `LoadNodes`: hands the buffered branches to `n` (group 0) and `q` (group 1).
fn LoadNodes(rtp: &mut RTree, n: NodeRef, q: NodeRef) {
    for i in 0..NODECARD_PLUS_1 {
        let b = rtp.split.BranchBuf[i];
        match rtp.split.Partitions[0].partition[i] {
            0 => {
                AddBranch(rtp, &b, n, None);
            }
            1 => {
                AddBranch(rtp, &b, q, None);
            }
            _ => {}
        }
    }
}

/// `InitPVars`.
fn InitPVars(rtp: &mut RTree) {
    let p = &mut rtp.split.Partitions[0];
    p.count = [0, 0];
    p.cover = [NullRect(), NullRect()];
    p.area = [0, 0];
    p.taken = [0; NODECARD_PLUS_1];
    p.partition = [-1; NODECARD_PLUS_1];
}
