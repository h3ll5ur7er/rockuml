//! `route.c`: fits a piecewise cubic Bézier spline to a polyline, splitting it until each piece stays clear of the
//! barriers.

use super::solvers::solve3;
use super::{PathplanContext, Pedge_t, Ppoint_t, Ppolyline_t, Pvector_t};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct tna_t {
    t: f64,
    a: [Ppoint_t; 2],
}

/// Fits a spline to the polyline `input` that does not cross `edges`, leaving and entering along `evs` (no
/// constraint where a vector is zero), and stores its control points in `output`.
pub fn Proutespline(
    zz: &mut PathplanContext,
    edges: &[Pedge_t],
    input: &Ppolyline_t,
    evs: [Pvector_t; 2],
    output: &mut Ppolyline_t,
) {
    let inps = &input.ps;
    let ev0 = normv(evs[0]);
    let ev1 = normv(evs[1]);
    output.ps.clear();
    output.ps.push(inps[0]);
    reallyroutespline(zz, edges, inps, ev0, ev1, &mut output.ps);
}

fn reallyroutespline(
    zz: &mut PathplanContext,
    edges: &[Pedge_t],
    inps: &[Ppoint_t],
    ev0: Ppoint_t,
    ev1: Ppoint_t,
    ops: &mut Vec<Ppoint_t>,
) {
    let inpn = inps.len();
    if zz.tnas.len() < inpn {
        zz.tnas.resize(inpn, tna_t::default());
    }
    let tnas = &mut zz.tnas;
    tnas[0].t = 0.0;
    for i in 1..inpn {
        tnas[i].t = tnas[i - 1].t + dist(inps[i], inps[i - 1]);
    }
    for i in 1..inpn {
        tnas[i].t /= tnas[inpn - 1].t;
    }
    for tna in &mut tnas[..inpn] {
        tna.a[0] = scale(ev0, B1(tna.t));
        tna.a[1] = scale(ev1, B2(tna.t));
    }
    let (p1, v1, p2, v2) = mkspline(inps, &tnas[..inpn], ev0, ev1);
    if splinefits(edges, p1, v1, p2, v2, inps, ops) {
        return;
    }
    let cp1 = add(p1, scale(v1, 1.0 / 3.0));
    let cp2 = sub(p2, scale(v2, 1.0 / 3.0));
    let mut maxd = -1.0;
    let mut maxi = None;
    for i in 1..inpn - 1 {
        let t = tnas[i].t;
        let p = Ppoint_t {
            x: B0(t) * p1.x + B1(t) * cp1.x + B2(t) * cp2.x + B3(t) * p2.x,
            y: B0(t) * p1.y + B1(t) * cp1.y + B2(t) * cp2.y + B3(t) * p2.y,
        };
        let d = dist(p, inps[i]);
        if d > maxd {
            maxd = d;
            maxi = Some(i);
        }
    }
    let spliti = maxi.expect("a polyline that needs splitting has an inner point");
    let splitv1 = normv(sub(inps[spliti], inps[spliti - 1]));
    let splitv2 = normv(sub(inps[spliti + 1], inps[spliti]));
    let splitv = normv(add(splitv1, splitv2));
    reallyroutespline(zz, edges, &inps[..=spliti], ev0, splitv, ops);
    reallyroutespline(zz, edges, &inps[spliti..], splitv, ev1, ops);
}

/// Returns the spline's end points and end vectors: `(sp0, sv0, sp1, sv1)`.
fn mkspline(
    inps: &[Ppoint_t],
    tnas: &[tna_t],
    ev0: Ppoint_t,
    ev1: Ppoint_t,
) -> (Ppoint_t, Pvector_t, Ppoint_t, Pvector_t) {
    let inpn = inps.len();
    let mut c = [[0.0; 2]; 2];
    let mut x = [0.0; 2];
    for (inp, tna) in inps.iter().zip(tnas) {
        c[0][0] += dot(tna.a[0], tna.a[0]);
        c[0][1] += dot(tna.a[0], tna.a[1]);
        c[1][0] = c[0][1];
        c[1][1] += dot(tna.a[1], tna.a[1]);
        let tmp = sub(
            *inp,
            add(
                scale(inps[0], B01(tna.t)),
                scale(inps[inpn - 1], B23(tna.t)),
            ),
        );
        x[0] += dot(tna.a[0], tmp);
        x[1] += dot(tna.a[1], tmp);
    }
    let det01 = c[0][0] * c[1][1] - c[1][0] * c[0][1];
    let det0X = c[0][0] * x[1] - c[0][1] * x[0];
    let detX1 = x[0] * c[1][1] - x[1] * c[0][1];
    let mut scale0 = 0.0;
    let mut scale3 = 0.0;
    if det01.abs() >= 1e-6 {
        scale0 = detX1 / det01;
        scale3 = det0X / det01;
    }
    if det01.abs() < 1e-6 || scale0 <= 0.0 || scale3 <= 0.0 {
        let d01 = dist(inps[0], inps[inpn - 1]) / 3.0;
        scale0 = d01;
        scale3 = d01;
    }
    (
        inps[0],
        scale(ev0, scale0),
        inps[inpn - 1],
        scale(ev1, scale3),
    )
}

fn dist_n(p: &[Ppoint_t]) -> f64 {
    let mut rv = 0.0;
    for w in p.windows(2) {
        rv +=
            ((w[1].x - w[0].x) * (w[1].x - w[0].x) + (w[1].y - w[0].y) * (w[1].y - w[0].y)).sqrt();
    }
    rv
}

/// Appends the spline's last three control points to `ops` if it stays clear of the barriers.
fn splinefits(
    edges: &[Pedge_t],
    pa: Ppoint_t,
    va: Pvector_t,
    pb: Ppoint_t,
    vb: Pvector_t,
    inps: &[Ppoint_t],
    ops: &mut Vec<Ppoint_t>,
) -> bool {
    let forceflag = inps.len() == 2;
    let mut first = true;
    let mut a = 4.0;
    let mut b = 4.0;
    loop {
        let sps = [
            pa,
            Ppoint_t {
                x: pa.x + a * va.x / 3.0,
                y: pa.y + a * va.y / 3.0,
            },
            Ppoint_t {
                x: pb.x - b * vb.x / 3.0,
                y: pb.y - b * vb.y / 3.0,
            },
            pb,
        ];

        // shortcuts (paths shorter than the shortest path) not allowed - they must be outside the constraint
        // polygon. this can happen if the candidate spline intersects the constraint polygon exactly on sides or
        // vertices. maybe this could be more elegant, but it solves the immediate problem. we could also try
        // jittering the constraint polygon, or computing the candidate spline more carefully, for example using
        // the path. SCN
        if first && (dist_n(&sps) < (dist_n(inps) - 1E-3)) {
            return false;
        }
        first = false;

        if splineisinside(edges, &sps) || (a == 0.0 && b == 0.0 && forceflag) {
            ops.extend_from_slice(&sps[1..]);
            return true;
        }
        if a == 0.0 && b == 0.0 {
            return false;
        }
        if a > 0.01 {
            a /= 2.0;
            b /= 2.0;
        } else {
            a = 0.0;
            b = 0.0;
        }
    }
}

fn splineisinside(edges: &[Pedge_t], sps: &[Ppoint_t; 4]) -> bool {
    let mut roots = [0.0; 4];
    for edge in edges {
        let lps = [edge.a, edge.b];
        let rootn = splineintersectsline(sps, &lps, &mut roots);
        if rootn == 4 {
            continue;
        }
        for &t in &roots[..rootn] {
            if t < 1E-6 || t > 1.0 - 1E-6 {
                continue;
            }
            let td = t * t * t;
            let tc = 3.0 * t * t * (1.0 - t);
            let tb = 3.0 * t * (1.0 - t) * (1.0 - t);
            let ta = (1.0 - t) * (1.0 - t) * (1.0 - t);
            let ip = Ppoint_t {
                x: ta * sps[0].x + tb * sps[1].x + tc * sps[2].x + td * sps[3].x,
                y: ta * sps[0].y + tb * sps[1].y + tc * sps[2].y + td * sps[3].y,
            };
            if DISTSQ(ip, lps[0]) < 1E-3 || DISTSQ(ip, lps[1]) < 1E-3 {
                continue;
            }
            return false;
        }
    }
    true
}

/// Returns the number of parameters in [0, 1] where the spline meets the line segment, stored in `roots`, or 4 if
/// they meet everywhere.
fn splineintersectsline(sps: &[Ppoint_t; 4], lps: &[Ppoint_t; 2], roots: &mut [f64; 4]) -> usize {
    let mut scoeff = [0.0; 4];
    let mut xroots = [0.0; 3];
    let mut yroots = [0.0; 3];
    let mut rootn = 0;
    let xcoeff = [lps[0].x, lps[1].x - lps[0].x];
    let ycoeff = [lps[0].y, lps[1].y - lps[0].y];
    if xcoeff[1] == 0.0 {
        if ycoeff[1] == 0.0 {
            points2coeff(sps[0].x, sps[1].x, sps[2].x, sps[3].x, &mut scoeff);
            scoeff[0] -= xcoeff[0];
            let xrootn = solve3(&scoeff, &mut xroots);
            points2coeff(sps[0].y, sps[1].y, sps[2].y, sps[3].y, &mut scoeff);
            scoeff[0] -= ycoeff[0];
            let yrootn = solve3(&scoeff, &mut yroots);
            if xrootn == 4 {
                if yrootn == 4 {
                    return 4;
                }
                for &yroot in &yroots[..yrootn] {
                    addroot(yroot, roots, &mut rootn);
                }
            } else if yrootn == 4 {
                for &xroot in &xroots[..xrootn] {
                    addroot(xroot, roots, &mut rootn);
                }
            } else {
                for &xroot in &xroots[..xrootn] {
                    for &yroot in &yroots[..yrootn] {
                        if xroot == yroot {
                            addroot(xroot, roots, &mut rootn);
                        }
                    }
                }
            }
            rootn
        } else {
            points2coeff(sps[0].x, sps[1].x, sps[2].x, sps[3].x, &mut scoeff);
            scoeff[0] -= xcoeff[0];
            let xrootn = solve3(&scoeff, &mut xroots);
            if xrootn == 4 {
                return 4;
            }
            for &tv in &xroots[..xrootn] {
                if (0.0..=1.0).contains(&tv) {
                    points2coeff(sps[0].y, sps[1].y, sps[2].y, sps[3].y, &mut scoeff);
                    let mut sv = scoeff[0] + tv * (scoeff[1] + tv * (scoeff[2] + tv * scoeff[3]));
                    sv = (sv - ycoeff[0]) / ycoeff[1];
                    if (0.0..=1.0).contains(&sv) {
                        addroot(tv, roots, &mut rootn);
                    }
                }
            }
            rootn
        }
    } else {
        let rat = ycoeff[1] / xcoeff[1];
        points2coeff(
            sps[0].y - rat * sps[0].x,
            sps[1].y - rat * sps[1].x,
            sps[2].y - rat * sps[2].x,
            sps[3].y - rat * sps[3].x,
            &mut scoeff,
        );
        scoeff[0] += rat * xcoeff[0] - ycoeff[0];
        let xrootn = solve3(&scoeff, &mut xroots);
        if xrootn == 4 {
            return 4;
        }
        for &tv in &xroots[..xrootn] {
            if (0.0..=1.0).contains(&tv) {
                points2coeff(sps[0].x, sps[1].x, sps[2].x, sps[3].x, &mut scoeff);
                let mut sv = scoeff[0] + tv * (scoeff[1] + tv * (scoeff[2] + tv * scoeff[3]));
                sv = (sv - xcoeff[0]) / xcoeff[1];
                if (0.0..=1.0).contains(&sv) {
                    addroot(tv, roots, &mut rootn);
                }
            }
        }
        rootn
    }
}

fn points2coeff(v0: f64, v1: f64, v2: f64, v3: f64, coeff: &mut [f64; 4]) {
    coeff[3] = v3 + 3.0 * v1 - (v0 + 3.0 * v2);
    coeff[2] = 3.0 * v0 + 3.0 * v2 - 6.0 * v1;
    coeff[1] = 3.0 * (v1 - v0);
    coeff[0] = v0;
}

fn addroot(root: f64, roots: &mut [f64; 4], rootnp: &mut usize) {
    if (0.0..=1.0).contains(&root) {
        roots[*rootnp] = root;
        *rootnp += 1;
    }
}

fn normv(mut v: Pvector_t) -> Pvector_t {
    let mut d = v.x * v.x + v.y * v.y;
    if d > 1e-6 {
        d = d.sqrt();
        v.x /= d;
        v.y /= d;
    }
    v
}

fn add(p1: Ppoint_t, p2: Ppoint_t) -> Ppoint_t {
    Ppoint_t {
        x: p1.x + p2.x,
        y: p1.y + p2.y,
    }
}

fn sub(p1: Ppoint_t, p2: Ppoint_t) -> Ppoint_t {
    Ppoint_t {
        x: p1.x - p2.x,
        y: p1.y - p2.y,
    }
}

fn dist(p1: Ppoint_t, p2: Ppoint_t) -> f64 {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    (dx * dx + dy * dy).sqrt()
}

fn scale(p: Ppoint_t, c: f64) -> Ppoint_t {
    Ppoint_t {
        x: p.x * c,
        y: p.y * c,
    }
}

fn dot(p1: Ppoint_t, p2: Ppoint_t) -> f64 {
    p1.x * p2.x + p1.y * p2.y
}

fn B0(t: f64) -> f64 {
    let tmp = 1.0 - t;
    tmp * tmp * tmp
}

fn B1(t: f64) -> f64 {
    let tmp = 1.0 - t;
    3.0 * t * tmp * tmp
}

fn B2(t: f64) -> f64 {
    let tmp = 1.0 - t;
    3.0 * t * t * tmp
}

fn B3(t: f64) -> f64 {
    t * t * t
}

fn B01(t: f64) -> f64 {
    let tmp = 1.0 - t;
    tmp * tmp * (tmp + 3.0 * t)
}

fn B23(t: f64) -> f64 {
    let tmp = 1.0 - t;
    t * t * (3.0 * tmp + t)
}

/// The squared distance between two points (Graphviz's `DISTSQ` macro).
fn DISTSQ(a: Ppoint_t, b: Ppoint_t) -> f64 {
    ((a.x - b.x) * (a.x - b.x)) + ((a.y - b.y) * (a.y - b.y))
}
