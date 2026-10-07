//! `xlabels.c`: places external labels next to their objects, avoiding the other objects and labels.
//!
//! The objects go into a bag ordered by the Hilbert index of their centres (a `Dtobag` keyed by `icompare`, so
//! equal keys come out in the splay tree's order), and from there into an R-tree in that order. Then each label
//! tries the eight positions around its object and keeps the one with the least overlap.

use crate::cdt::{DT_OBAG, Dt};
use crate::core::ids::TextlabelId;
use crate::h::{boxf, point, pointf};

use super::index::{RTreeInsert, RTreeOpen, RTreeSearch};
use super::{Branch_t, Child, RTree, Rect_t};

/// `xlabel_t`: a label to place, of size `sz`; `pos` is its lower left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct xlabel_t {
    pub sz: pointf,
    pub pos: pointf,
    pub lbl: Option<TextlabelId>,
    pub set: i32,
}

/// `object_t`: an obstacle (a node or a placed label) at `pos` (lower left) of size `sz`, or the point an
/// unplaced label belongs to, with the label's index in `lbl`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct object_t {
    pub pos: pointf,
    pub sz: pointf,
    pub lbl: Option<usize>,
}

/// `label_params_t`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct label_params_t {
    pub bb: boxf,
    pub force: bool,
}

/// `BestPos_t`: a candidate position, with the number and the area of its overlaps.
#[derive(Clone, Copy, Debug, Default)]
struct BestPos_t {
    n: i32,
    area: f64,
    pos: pointf,
}

/// `HDict_t`: an object in the Hilbert bag. Its `d.child` is the object's index.
#[derive(Clone, Copy, Debug)]
struct HDict_t {
    key: i32,
    d: Branch_t,
}

/// `XLabels_t`.
struct XLabels_t<'a> {
    objs: &'a [object_t],
    n_objs: i32,
    lbls: &'a mut [xlabel_t],
    params: &'a label_params_t,
    /// The bag holds indices into `hdicts`.
    hdx: Dt<usize, i32>,
    hdicts: Vec<HDict_t>,
    spdx: RTree,
}

impl XLabels_t<'_> {
    fn obj(&self, i: usize) -> &object_t {
        &self.objs[i]
    }

    /// `objp->lbl[0]`.
    fn lbl(&self, objp: usize) -> &xlabel_t {
        &self.lbls[self.objs[objp].lbl.expect("object without a label")]
    }

    fn lbl_mut(&mut self, objp: usize) -> &mut xlabel_t {
        &mut self.lbls[self.objs[objp].lbl.expect("object without a label")]
    }
}

/// `xlnew`.
fn xlnew<'a>(
    objs: &'a [object_t],
    n_objs: i32,
    lbls: &'a mut [xlabel_t],
    params: &'a label_params_t,
) -> XLabels_t<'a> {
    XLabels_t {
        objs,
        n_objs,
        lbls,
        params,
        hdx: Dt::dtopen(DT_OBAG),
        hdicts: Vec::new(),
        spdx: RTreeOpen(),
    }
}

/// `floorLog2`: the position of the highest set bit; -1 for 0, and 0 for negative numbers.
fn floorLog2(mut n: i32) -> i32 {
    let mut pos = 0;
    if n == 0 {
        return -1;
    }
    if n >= 1 << 16 {
        n >>= 16;
        pos += 16;
    }
    if n >= 1 << 8 {
        n >>= 8;
        pos += 8;
    }
    if n >= 1 << 4 {
        n >>= 4;
        pos += 4;
    }
    if n >= 1 << 2 {
        n >>= 2;
        pos += 2;
    }
    if n >= 1 << 1 {
        pos += 1;
    }
    pos
}

/// `xlhorder`: the order of the Hilbert curve that covers the bounding box.
#[allow(clippy::similar_names, reason = "Graphviz's names")]
fn xlhorder(xlp: &XLabels_t<'_>) -> i32 {
    let maxx = xlp.params.bb.UR.x;
    let maxy = xlp.params.bb.UR.y;
    floorLog2(if maxx > maxy {
        maxx as i32
    } else {
        maxy as i32
    }) + 1
}

/// `hd_hil_s_from_xy`: the index of `p` along a Hilbert curve of order `n`, in Java's wrapping `int`.
pub fn hd_hil_s_from_xy(p: point, n: i32) -> i32 {
    let mut x = p.x;
    let mut y = p.y;
    let mut s: i32 = 0;
    for i in (0..n).rev() {
        let xi = (x >> i) & 1;
        let yi = (y >> i) & 1;
        s = s
            .wrapping_mul(4)
            .wrapping_add(xi.wrapping_mul(2))
            .wrapping_add(xi ^ yi);
        x ^= y;
        y ^= x & (yi - 1);
        x ^= y;
        x ^= -xi & (yi - 1);
        y ^= -xi & (yi - 1);
    }
    s
}

/// `aabbaabb`: the area of the intersection of two rectangles, 0 when they are disjoint.
#[allow(clippy::similar_names, reason = "Graphviz's names")]
fn aabbaabb(r: &Rect_t, s: &Rect_t) -> f64 {
    if r.boundary[2] < s.boundary[0] || r.boundary[0] > s.boundary[2] {
        return 0.0;
    }
    if r.boundary[3] < s.boundary[1] || r.boundary[1] > s.boundary[3] {
        return 0.0;
    }
    let iminx = f64::from(r.boundary[0].max(s.boundary[0]));
    let iminy = f64::from(r.boundary[1].max(s.boundary[1]));
    let imaxx = f64::from(r.boundary[2].min(s.boundary[2]));
    let imaxy = f64::from(r.boundary[3].min(s.boundary[3]));
    (imaxx - iminx) * (imaxy - iminy)
}

/// `lblenclosing`: whether `objp1` lies inside `objp`'s label. Graphviz compares x with the label's height too.
fn lblenclosing(xlp: &XLabels_t<'_>, objp: usize, objp1: usize) -> bool {
    let Some(lbl) = xlp.obj(objp).lbl else {
        return false;
    };
    let lp = &xlp.lbls[lbl];
    let p1 = xlp.obj(objp1).pos;
    p1.x > lp.pos.x && p1.x < (lp.pos.x + lp.sz.y) && p1.y > lp.pos.y && p1.y < (lp.pos.y + lp.sz.y)
}

/// `objp2rect`: the object's rectangle, truncated to integers.
fn objp2rect(op: &object_t) -> Rect_t {
    Rect_t {
        boundary: [
            op.pos.x as i32,
            op.pos.y as i32,
            (op.pos.x + op.sz.x) as i32,
            (op.pos.y + op.sz.y) as i32,
        ],
    }
}

/// `objplp2rect`: the rectangle of the object's label, truncated to integers.
fn objplp2rect(xlp: &XLabels_t<'_>, objp: usize) -> Rect_t {
    let lp = xlp.lbl(objp);
    Rect_t {
        boundary: [
            lp.pos.x as i32,
            lp.pos.y as i32,
            (lp.pos.x + lp.sz.x) as i32,
            (lp.pos.y + lp.sz.y) as i32,
        ],
    }
}

/// `objplpmks`: the object grown by its label's size on every side: everywhere its label could go.
fn objplpmks(xlp: &XLabels_t<'_>, objp: usize) -> Rect_t {
    let op = xlp.obj(objp);
    let p = op.lbl.map_or(pointf { x: 0.0, y: 0.0 }, |l| xlp.lbls[l].sz);
    Rect_t {
        boundary: [
            (op.pos.x - p.x).floor() as i32,
            (op.pos.y - p.y).floor() as i32,
            (op.pos.x + op.sz.x + p.x).ceil() as i32,
            (op.pos.y + op.sz.y + p.y).ceil() as i32,
        ],
    }
}

/// `getintrsxi`: which of the 9 sectors around `op` the object `cp` is in. Smetana only implements the case of
/// an unplaced label, -1; a label being placed is never set, so that is the only case reached.
fn getintrsxi(xlp: &XLabels_t<'_>, op: usize, cp: usize) -> i32 {
    let i = -1;
    if xlp.lbl(op).set == 0 || xlp.lbl(cp).set == 0 {
        return i;
    }
    unimplemented!("getintrsxi for placed labels")
}

/// `recordointrsx`: the overlap area to count for object `cp`, remembering the largest intersecting object per
/// sector in `intrsx`.
fn recordointrsx(
    xlp: &XLabels_t<'_>,
    op: usize,
    cp: usize,
    rp: &Rect_t,
    a: f64,
    intrsx: &mut [Option<usize>; 9],
) -> f64 {
    let mut i = getintrsxi(xlp, op, cp);
    if i < 0 {
        i = 5;
    }
    let i = i as usize;
    if let Some(prev) = intrsx[i] {
        let mut maxa = 0.0;
        let srect = objp2rect(xlp.obj(prev));
        let sa = aabbaabb(rp, &srect);
        if sa > a {
            maxa = sa;
        }
        if xlp.obj(prev).lbl.is_some() {
            let srect = objplp2rect(xlp, prev);
            let sa = aabbaabb(rp, &srect);
            if sa > a {
                maxa = if sa > maxa { sa } else { maxa };
            }
        }
        if maxa > 0.0 {
            return maxa;
        }
        intrsx[i] = Some(cp);
        return a;
    }
    intrsx[i] = Some(cp);
    a
}

/// `recordlintrsx`: the same for the label of `cp`; Graphviz's two functions have identical bodies.
fn recordlintrsx(
    xlp: &XLabels_t<'_>,
    op: usize,
    cp: usize,
    rp: &Rect_t,
    a: f64,
    intrsx: &mut [Option<usize>; 9],
) -> f64 {
    recordointrsx(xlp, op, cp, rp, a, intrsx)
}

/// `xlintersections`: how many objects and labels the label of `objp` overlaps at its current position, and how
/// much.
fn xlintersections(xlp: &XLabels_t<'_>, objp: usize, intrsx: &mut [Option<usize>; 9]) -> BestPos_t {
    let mut bp = BestPos_t {
        n: 0,
        area: 0.0,
        pos: xlp.lbl(objp).pos,
    };
    for i in 0..xlp.n_objs as usize {
        if objp == i {
            continue;
        }
        if xlp.obj(i).sz.x > 0.0 && xlp.obj(i).sz.y > 0.0 {
            continue;
        }
        if lblenclosing(xlp, objp, i) {
            bp.n += 1;
        }
    }
    let rect = objplp2rect(xlp, objp);
    let llp = RTreeSearch(&xlp.spdx, xlp.spdx.root, &rect);
    if llp.is_empty() {
        return bp;
    }
    for ilp in &llp {
        let Some(Child::Data(cp)) = ilp.child else {
            panic!("an R-tree leaf holds a node");
        };
        if cp == objp {
            continue;
        }
        let srect = objp2rect(xlp.obj(cp));
        let a = aabbaabb(&rect, &srect);
        if a > 0.0 {
            let ra = recordointrsx(xlp, objp, cp, &rect, a, intrsx);
            bp.n += 1;
            bp.area += ra;
        }
        if xlp.obj(cp).lbl.is_none() || xlp.lbl(cp).set == 0 {
            continue;
        }
        let srect = objplp2rect(xlp, cp);
        let a = aabbaabb(&rect, &srect);
        if a > 0.0 {
            let ra = recordlintrsx(xlp, objp, cp, &rect, a, intrsx);
            bp.n += 1;
            bp.area += ra;
        }
    }
    bp
}

/// One step of `xladjust`: counts the overlaps of the label of `objp` where it is now. Returns the position if it
/// is free, else keeps it in `bp` if it overlaps less.
fn xlcandidate(
    xlp: &XLabels_t<'_>,
    objp: usize,
    intrsx: &mut [Option<usize>; 9],
    bp: &mut BestPos_t,
) -> Option<BestPos_t> {
    let nbp = xlintersections(xlp, objp, intrsx);
    if nbp.n == 0 {
        return Some(nbp);
    }
    if nbp.area < bp.area {
        *bp = nbp;
    }
    None
}

/// `xladjust`: tries the label of `objp` at the corners and sides of the object and returns the first position
/// without overlaps, or else the one with the least overlapping area. Leaves the label at the last position tried.
fn xladjust(xlp: &mut XLabels_t<'_>, objp: usize) -> BestPos_t {
    let op = *xlp.obj(objp);
    let sz = xlp.lbl(objp).sz;
    let xincr = (2.0 * sz.x + op.sz.x) / 8.0;
    let mut intrsx: [Option<usize>; 9] = [None; 9];

    // Left of the object: at the top, the middle and the bottom.
    xlp.lbl_mut(objp).pos.x = op.pos.x - sz.x;
    xlp.lbl_mut(objp).pos.y = op.pos.y + op.sz.y;
    let mut bp = xlintersections(xlp, objp, &mut intrsx);
    if bp.n == 0 {
        return bp;
    }
    xlp.lbl_mut(objp).pos.y = op.pos.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }
    xlp.lbl_mut(objp).pos.y = op.pos.y - sz.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }
    // Above and below.
    xlp.lbl_mut(objp).pos.x = op.pos.x;
    xlp.lbl_mut(objp).pos.y = op.pos.y + op.sz.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }
    xlp.lbl_mut(objp).pos.y = op.pos.y - sz.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }
    // Right of the object: at the top, the middle and the bottom.
    xlp.lbl_mut(objp).pos.x = op.pos.x + op.sz.x;
    xlp.lbl_mut(objp).pos.y = op.pos.y + op.sz.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }
    xlp.lbl_mut(objp).pos.y = op.pos.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }
    xlp.lbl_mut(objp).pos.y = op.pos.y - sz.y;
    if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
        return free;
    }

    // Only sector 5 is ever recorded (see getintrsxi).
    if intrsx[6].is_some()
        || intrsx[7].is_some()
        || intrsx[8].is_some()
        || intrsx[3].is_some()
        || intrsx[0].is_some()
    {
        unimplemented!("xladjust: sliding along the top or the left side");
    }

    xlp.lbl_mut(objp).pos.x = op.pos.x + op.sz.x;
    xlp.lbl_mut(objp).pos.y = op.pos.y - sz.y;
    if intrsx[2].is_some()
        || intrsx[1].is_some()
        || intrsx[0].is_some()
        || intrsx[5].is_some()
        || intrsx[8].is_some()
    {
        if intrsx[1].is_none() && intrsx[0].is_none() {
            // Slide along the bottom, from right to left.
            xlp.lbl_mut(objp).pos.x = op.pos.x + op.sz.x;
            xlp.lbl_mut(objp).pos.y = op.pos.y - sz.y;
            while xlp.lbl(objp).pos.x >= (op.pos.x - sz.x) {
                if let Some(free) = xlcandidate(xlp, objp, &mut intrsx, &mut bp) {
                    return free;
                }
                xlp.lbl_mut(objp).pos.x -= xincr;
            }
        }
        if intrsx[5].is_none() && intrsx[8].is_none() {
            unimplemented!("xladjust: sliding along the right side");
        }
    }
    bp
}

/// `xlhdxload`: puts every object into the Hilbert bag, keyed by the centre of [`objplpmks`].
fn xlhdxload(xlp: &mut XLabels_t<'_>) -> i32 {
    let order = xlhorder(xlp);
    for i in 0..xlp.n_objs as usize {
        let rect = objplpmks(xlp, i);
        let b = rect.boundary;
        let pi = point {
            x: b[0].wrapping_add(b[2].wrapping_sub(b[0]) / 2),
            y: b[1].wrapping_add(b[3].wrapping_sub(b[1]) / 2),
        };
        let hp = HDict_t {
            key: hd_hil_s_from_xy(pi, order),
            d: Branch_t {
                rect,
                child: Some(Child::Data(i)),
            },
        };
        xlp.hdicts.push(hp);
        if xlp.hdx.dtinsert(xlp.hdicts.len() - 1, hp.key).is_none() {
            return -1;
        }
    }
    0
}

/// `xlspdxload`: inserts the objects into the R-tree in Hilbert order.
fn xlspdxload(xlp: &mut XLabels_t<'_>) -> i32 {
    let mut op = xlp.hdx.dtfirst();
    while let Some(o) = op {
        let hp = xlp.hdicts[o];
        let Some(Child::Data(data)) = hp.d.child else {
            unreachable!("a Hilbert entry holds its object")
        };
        let mut root = xlp.spdx.root;
        RTreeInsert(&mut xlp.spdx, &hp.d.rect, data, &mut root, 0);
        xlp.spdx.root = root;
        op = xlp.hdx.dtnext(o, hp.key);
    }
    0
}

/// `xlinitialize`.
fn xlinitialize(xlp: &mut XLabels_t<'_>) -> i32 {
    let r = xlhdxload(xlp);
    if r < 0 {
        return r;
    }
    let r = xlspdxload(xlp);
    if r < 0 {
        return r;
    }
    0
}

/// `placeLabels`: places the labels of the objects that have one, setting their `pos` and `set`. Returns 1 when
/// some label overlaps something and `params.force` is off (it stays unset), else 0.
pub fn placeLabels(
    objs: &[object_t],
    n_objs: i32,
    lbls: &mut [xlabel_t],
    params: &label_params_t,
) -> i32 {
    let mut xlp = xlnew(objs, n_objs, lbls, params);
    let r = xlinitialize(&mut xlp);
    if r < 0 {
        return r;
    }
    let mut r = 0;
    for (i, obj) in objs.iter().enumerate().take(n_objs as usize) {
        if obj.lbl.is_none() {
            continue;
        }
        let bp = xladjust(&mut xlp, i);
        let lp = xlp.lbl_mut(i);
        if bp.n == 0 {
            lp.set = 1;
        } else if bp.area == 0.0 || params.force {
            lp.pos.x = bp.pos.x;
            lp.pos.y = bp.pos.y;
            lp.set = 1;
        } else {
            r = 1;
        }
    }
    r
}
