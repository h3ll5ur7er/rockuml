//! `routespl.c`: routes an edge through its corridor of boxes with the path planner, then shrinks the boxes to
//! the space the spline takes, so that later edges can use the rest.
//!
//! C keeps the polygon, barrier and result arrays in statics that grow across calls (`routesplinesinit` and
//! `routesplinesterm` allocate and free them); no value outlives a call, so here they are local vectors.

use crate::cgraph::{aghead, agtail};
use crate::core::Globals;
use crate::core::consts::{INT_MAX, INT_MIN, NORMAL};
use crate::core::jmath::{cos, sin};
use crate::h::{boxf, path, pointf};
use crate::pathplan::{
    Pedge_t, Ppoint_t, Ppoly_t, Ppolyline_t, Proutespline, Pshortestpath, make_polyline,
};

fn to_ppoint(p: pointf) -> Ppoint_t {
    Ppoint_t { x: p.x, y: p.y }
}

fn to_pointf(p: Ppoint_t) -> pointf {
    pointf { x: p.x, y: p.y }
}

/// The polygon's sides, the barriers the spline must not cross.
fn sides(poly: &Ppoly_t) -> Vec<Pedge_t> {
    let pn = poly.ps.len();
    (0..pn)
        .map(|i| Pedge_t {
            a: poly.ps[i],
            b: poly.ps[(i + 1) % pn],
        })
        .collect()
}

/// `simpleSplineRoute`: a spline (or with `polyline`, a polyline) from `tp` to `hp` inside the polygon `poly`;
/// `None` if the path planner finds no path.
pub fn simpleSplineRoute(
    zz: &mut Globals,
    tp: pointf,
    hp: pointf,
    poly: &[pointf],
    polyline: bool,
) -> Option<Vec<pointf>> {
    let poly = Ppoly_t {
        ps: poly.iter().copied().map(to_ppoint).collect(),
    };
    let eps = [to_ppoint(tp), to_ppoint(hp)];
    let mut pl = Ppolyline_t::default();
    if Pshortestpath(&mut zz.pathplan, &poly, eps, &mut pl).is_err() {
        return None;
    }
    let mut spl = Ppolyline_t::default();
    if polyline {
        make_polyline(&pl, &mut spl);
    } else {
        let evs = [Ppoint_t::default(); 2];
        Proutespline(&mut zz.pathplan, &sides(&poly), &pl, evs, &mut spl);
    }
    Some(spl.ps.into_iter().map(to_pointf).collect())
}

/// `limitBoxes`: shrinks the boxes' x extents to the samples of the spline `pps` that fall in them, `delta`
/// samples per box and Bézier piece. The boxes start out inverted (`LL.x` at `INT_MAX`, `UR.x` at `INT_MIN`).
fn limitBoxes(boxes: &mut [boxf], boxn: i32, pps: &[pointf], pn: i32, delta: i32) {
    let num_div = delta * boxn;
    let mut splinepi = 0;
    while splinepi + 3 < pn {
        let i = splinepi as usize;
        for si in 0..=num_div {
            let t = f64::from(si) / f64::from(num_div);
            let (mut s0, mut s1, mut s2, s3) = (pps[i], pps[i + 1], pps[i + 2], pps[i + 3]);
            s0.x = s0.x + t * (s1.x - s0.x);
            s0.y = s0.y + t * (s1.y - s0.y);
            s1.x = s1.x + t * (s2.x - s1.x);
            s1.y = s1.y + t * (s2.y - s1.y);
            s2.x = s2.x + t * (s3.x - s2.x);
            s2.y = s2.y + t * (s3.y - s2.y);
            s0.x = s0.x + t * (s1.x - s0.x);
            s0.y = s0.y + t * (s1.y - s0.y);
            s1.x = s1.x + t * (s2.x - s1.x);
            s1.y = s1.y + t * (s2.y - s1.y);
            s0.x = s0.x + t * (s1.x - s0.x);
            s0.y = s0.y + t * (s1.y - s0.y);
            for b in &mut boxes[..boxn as usize] {
                // Graphviz's fudge for 32-bit machines.
                if s0.y <= b.UR.y + 0.0001 && s0.y >= b.LL.y - 0.0001 {
                    if b.LL.x > s0.x {
                        b.LL.x = s0.x;
                    }
                    if b.UR.x < s0.x {
                        b.UR.x = s0.x;
                    }
                }
            }
        }
        splinepi += 3;
    }
}

/// Fills a box's x extent with the inverted bounds `limitBoxes` starts from.
fn invert_x(boxes: &mut [boxf]) {
    for b in boxes {
        b.LL.x = f64::from(INT_MAX);
        b.UR.x = f64::from(INT_MIN);
    }
}

/// `_routesplines`: routes `pp` and returns the spline's control points (with `polyline`, a polyline drawn as a
/// spline), or `None` if the path planner fails. The path's boxes are left shrunk around the result.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
fn _routesplines(zz: &mut Globals, pp: &mut path, polyline: bool) -> Option<Vec<pointf>> {
    let mut realedge = pp.data.expect("path without edge");
    while zz.ed(realedge).edge_type != NORMAL {
        realedge = zz.ed(realedge).to_orig.unwrap_or_else(|| {
            unimplemented!("in routesplines, cannot find NORMAL edge");
        });
    }
    let boxn = pp.nbox;
    checkpath(boxn, pp);
    let nb = boxn as usize;
    let boxes = &mut pp.boxes[..nb];
    let flip = nb > 1 && boxes[0].LL.y > boxes[1].LL.y;
    if flip {
        for b in boxes.iter_mut() {
            let v = b.UR.y;
            b.UR.y = -b.LL.y;
            b.LL.y = -v;
        }
    }
    let mut polypoints: Vec<pointf> = Vec::with_capacity(nb * 8);
    let mut push = |x: f64, y: f64| polypoints.push(pointf { x, y });
    if agtail(zz, realedge) == aghead(zz, realedge) {
        unimplemented!("1izvmtfwbnl5xq4u2x5fdraxp: in routesplines, edge is a loop");
    }
    // The path goes down only, or up, right and down.
    for bi in 0..nb {
        let mut prev = 0;
        let mut next = 0;
        if bi > 0 {
            prev = if boxes[bi].LL.y > boxes[bi - 1].LL.y {
                -1
            } else {
                1
            };
        }
        if bi < nb - 1 {
            next = if boxes[bi + 1].LL.y > boxes[bi].LL.y {
                1
            } else {
                -1
            };
        }
        let b = boxes[bi];
        if prev != next {
            if next == -1 || prev == 1 {
                push(b.LL.x, b.UR.y);
                push(b.LL.x, b.LL.y);
            } else {
                push(b.UR.x, b.LL.y);
                push(b.UR.x, b.UR.y);
            }
        } else if prev == 0 {
            unimplemented!("2bfai79qe7cec0rljrn56jg2f: in routesplines, a single box");
        } else if !(prev == -1 && next == -1) {
            unimplemented!(
                "cgpvvfb9phbipyhij0cjh1nmi: in routesplines, illegal values of prev and next"
            );
        }
    }
    for bi in (0..nb).rev() {
        let mut prev = 0;
        let mut next = 0;
        if bi < nb - 1 {
            prev = if boxes[bi].LL.y > boxes[bi + 1].LL.y {
                -1
            } else {
                1
            };
        }
        if bi > 0 {
            next = if boxes[bi - 1].LL.y > boxes[bi].LL.y {
                1
            } else {
                -1
            };
        }
        let b = boxes[bi];
        if prev != next {
            if next == -1 || prev == 1 {
                push(b.LL.x, b.UR.y);
                push(b.LL.x, b.LL.y);
            } else {
                push(b.UR.x, b.LL.y);
                push(b.UR.x, b.UR.y);
            }
        } else if prev == 0 {
            unimplemented!("ya84m81ogarx28l99om39lba: in routesplines, a single box");
        } else {
            if !(prev == -1 && next == -1) {
                unimplemented!(
                    "87y5d0ts6xdjyx905bha50f3s: in routesplines, illegal values of prev and next"
                );
            }
            push(b.UR.x, b.LL.y);
            push(b.UR.x, b.UR.y);
            push(b.LL.x, b.UR.y);
            push(b.LL.x, b.LL.y);
        }
    }
    if flip {
        for b in boxes.iter_mut() {
            // Graphviz truncates on the way back.
            let v = b.UR.y as i32;
            b.UR.y = -b.LL.y;
            b.LL.y = f64::from(-v);
        }
        for p in &mut polypoints {
            p.y = -p.y;
        }
    }
    invert_x(boxes);
    let poly = Ppoly_t {
        ps: polypoints.into_iter().map(to_ppoint).collect(),
    };
    let eps = [to_ppoint(pp.start.p), to_ppoint(pp.end.p)];
    let mut pl = Ppolyline_t::default();
    if Pshortestpath(&mut zz.pathplan, &poly, eps, &mut pl).is_err() {
        // Smetana prints "in routesplines, Pshortestpath failed".
        return None;
    }
    let mut spl = Ppolyline_t::default();
    if polyline {
        make_polyline(&pl, &mut spl);
    } else {
        let mut evs = [Ppoint_t::default(); 2];
        if pp.start.constrained {
            evs[0].x = cos(pp.start.theta);
            evs[0].y = sin(pp.start.theta);
        }
        if pp.end.constrained {
            evs[1].x = -cos(pp.end.theta);
            evs[1].y = -sin(pp.end.theta);
        }
        Proutespline(&mut zz.pathplan, &sides(&poly), &pl, evs, &mut spl);
    }
    let boxes = &mut pp.boxes[..nb];
    invert_x(boxes);
    let ps: Vec<pointf> = spl.ps.iter().copied().map(to_pointf).collect();
    let pn = ps.len() as i32;
    let mut delta = 10;
    let mut unbounded = true;
    let mut loopcnt = 0;
    while unbounded && loopcnt < 15 {
        limitBoxes(boxes, boxn, &ps, pn, delta);
        // A box too low for the samples to hit stays inverted: sample more finely.
        let missed = boxes
            .iter()
            .any(|b| b.LL.x == f64::from(INT_MAX) || b.UR.x == f64::from(INT_MIN));
        if missed {
            delta *= 2;
            if delta > INT_MAX / boxn {
                loopcnt = 15;
            }
        } else {
            unbounded = false;
        }
        loopcnt += 1;
    }
    if unbounded {
        // Smetana prints "Unable to reclaim box space in spline routing" and bounds the boxes by the shortest
        // path instead.
        let mut polyspl = Ppolyline_t::default();
        make_polyline(&pl, &mut polyspl);
        let polyspl: Vec<pointf> = polyspl.ps.into_iter().map(to_pointf).collect();
        limitBoxes(boxes, boxn, &polyspl, polyspl.len() as i32, 10);
    }
    Some(ps)
}

/// `routesplines`: the spline for path `pp`.
pub fn routesplines(zz: &mut Globals, pp: &mut path) -> Option<Vec<pointf>> {
    #[allow(clippy::used_underscore_items, reason = "Graphviz's name")]
    _routesplines(zz, pp, false)
}

/// `routepolylines`: the polyline for path `pp`, as a spline.
pub fn routepolylines(zz: &mut Globals, pp: &mut path) -> Option<Vec<pointf>> {
    #[allow(clippy::used_underscore_items, reason = "Graphviz's name")]
    _routesplines(zz, pp, true)
}

/// `overlap`: the length of the overlap of `[i0, i1]` and `[j0, j1]`, in whole points as C's `int` parameters
/// truncate them.
fn overlap(i0: f64, i1: f64, j0: f64, j1: f64) -> i32 {
    let (i0, i1, j0, j1) = (i0 as i32, i1 as i32, j0 as i32, j1 as i32);
    if i1 <= j0 || i0 >= j1 {
        return 0;
    }
    if j0 <= i0 && i0 <= j1 {
        return j1.wrapping_sub(i0);
    }
    if j0 <= i1 && i1 <= j1 {
        return i1.wrapping_sub(j0);
    }
    let a = i1.wrapping_sub(i0);
    let b = j1.wrapping_sub(j0);
    if a <= b { a } else { b }
}

/// `checkpath`: repairs the corridor: drops degenerate boxes (moving the rest up; the count the caller holds
/// stays), makes neighbours touch and not overlap, and moves the end points into the end boxes. The `u` branch
/// and the second pass, for boxes apart on both axes, are PlantUML's additions to Smetana.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
fn checkpath(boxn: i32, thepath: &mut path) {
    let boxes = &mut thepath.boxes;
    let mut i = 0;
    for bi in 0..boxn as usize {
        if (boxes[bi].LL.y - boxes[bi].UR.y).abs() < 0.01 {
            continue;
        }
        if (boxes[bi].LL.x - boxes[bi].UR.x).abs() < 0.01 {
            continue;
        }
        if i != bi {
            boxes[i] = boxes[bi];
        }
        i += 1;
    }
    let boxn = i;
    let ba = boxes[0];
    if ba.LL.x > ba.UR.x || ba.LL.y > ba.UR.y {
        unimplemented!("39tznwvf6k5lgj78jp32p0kfl: in checkpath, box 0 has LL coord > UR coord");
    }
    for bi in 0..boxn.saturating_sub(1) {
        let (head, tail) = boxes.split_at_mut(bi + 1);
        let ba = &mut head[bi];
        let bb = &mut tail[0];
        if bb.LL.x > bb.UR.x || bb.LL.y > bb.UR.y {
            unimplemented!("c8oodo0ge4n4dglb28fvf610v: in checkpath, box has LL coord > UR coord");
        }
        let mut l = i32::from(ba.UR.x < bb.LL.x);
        let mut r = i32::from(ba.LL.x > bb.UR.x);
        let mut d = i32::from(ba.UR.y < bb.LL.y);
        let mut u = i32::from(ba.LL.y > bb.UR.y);
        let errs = l + r + d + u;
        if errs > 0 {
            if l == 1 {
                let xy = ba.UR.x as i32;
                ba.UR.x = bb.LL.x;
                bb.LL.x = f64::from(xy);
                l = 0;
            } else if r == 1 {
                let xy = ba.LL.x as i32;
                ba.LL.x = bb.UR.x;
                bb.UR.x = f64::from(xy);
                r = 0;
            } else if d == 1 {
                let xy = ba.UR.y as i32;
                ba.UR.y = bb.LL.y;
                bb.LL.y = f64::from(xy);
                d = 0;
            } else if u == 1 {
                let xy = ba.LL.y as i32;
                ba.LL.y = bb.UR.y;
                bb.UR.y = f64::from(xy);
                u = 0;
            }
            for _ in 0..errs - 1 {
                if l == 1 {
                    let xy = f64::from(((ba.UR.x + bb.LL.x) / 2.0 + 0.5) as i32);
                    ba.UR.x = xy;
                    bb.LL.x = xy;
                    l = 0;
                } else if r == 1 {
                    let xy = f64::from(((ba.LL.x + bb.UR.x) / 2.0 + 0.5) as i32);
                    ba.LL.x = xy;
                    bb.UR.x = xy;
                    r = 0;
                } else if d == 1 {
                    let xy = f64::from(((ba.UR.y + bb.LL.y) / 2.0 + 0.5) as i32);
                    ba.UR.y = xy;
                    bb.LL.y = xy;
                    d = 0;
                } else if u == 1 {
                    let xy = f64::from(((ba.LL.y + bb.UR.y) / 2.0 + 0.5) as i32);
                    ba.LL.y = xy;
                    bb.UR.y = xy;
                    u = 0;
                }
            }
        }
        let xoverlap = overlap(ba.LL.x, ba.UR.x, bb.LL.x, bb.UR.x);
        let yoverlap = overlap(ba.LL.y, ba.UR.y, bb.LL.y, bb.UR.y);
        if xoverlap != 0 && yoverlap != 0 {
            if xoverlap < yoverlap {
                if ba.UR.x - ba.LL.x > bb.UR.x - bb.LL.x {
                    unimplemented!(
                        "5dqxf3gq05pjtobtnru1g2tuj: checkpath takes x space from the first box"
                    );
                } else if ba.UR.x < bb.UR.x {
                    bb.LL.x = ba.UR.x;
                } else {
                    bb.UR.x = ba.LL.x;
                }
            } else if ba.UR.y - ba.LL.y > bb.UR.y - bb.LL.y {
                if ba.UR.y < bb.UR.y {
                    ba.UR.y = bb.LL.y;
                } else {
                    ba.LL.y = bb.UR.y;
                }
            } else if ba.UR.y < bb.UR.y {
                bb.LL.y = ba.UR.y;
            } else {
                bb.UR.y = ba.LL.y;
            }
        }
    }
    let first = boxes[0];
    let last = boxes[boxn - 1];
    clamp_into(&mut thepath.start.p, first);
    clamp_into(&mut thepath.end.p, last);
}

/// Moves `p` onto the nearest point of `b` if it lies outside.
fn clamp_into(p: &mut pointf, b: boxf) {
    if p.x < b.LL.x {
        p.x = b.LL.x;
    }
    if p.x > b.UR.x {
        p.x = b.UR.x;
    }
    if p.y < b.LL.y {
        p.y = b.LL.y;
    }
    if p.y > b.UR.y {
        p.y = b.UR.y;
    }
}
