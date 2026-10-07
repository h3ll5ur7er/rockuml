//! `shortest.c`: the shortest path inside a polygon, by triangulating it and walking a funnel through the strip of
//! triangles between the endpoints.

use super::{PathplanContext, Ppoint_t, Ppoly_t, Ppolyline_t};

const ISCCW: i32 = 1;
const ISCW: i32 = 2;
const ISON: i32 = 3;

const DQ_FRONT: i32 = 1;
const DQ_BACK: i32 = 2;

/// Why `Pshortestpath` found no path. Graphviz returns these as negative codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathplanError {
    /// -1: the destination lies in no triangle of the polygon.
    DestinationOutside,
    /// -2: the polygon could not be triangulated (C's `longjmp`, Smetana's `PathplanAbort`).
    TriangulationFailed,
}

/// A point of the funnel: one of the polygon's (`pnls`) or one of the two endpoints (`epnls`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pnl {
    Poly(usize),
    End(usize),
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct pointnlink_t {
    pp: Ppoint_t,
    link: Option<Pnl>,
}

/// A triangle side. A polygon point is identified by its `pnls` index, as Smetana compares points by reference.
#[derive(Clone, Copy, Debug, Default)]
struct tedge_t {
    pnl0p: usize,
    pnl1p: usize,
    rtp: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct triangle_t {
    mark: i32,
    e: [tedge_t; 3],
}

#[derive(Debug, Default)]
struct deque_t {
    pnlps: Vec<Option<Pnl>>,
    fpnlpi: usize,
    lpnlpi: usize,
    apex: usize,
}

/// shortest.c's statics. The endpoints' `epnls` are locals in C, but funnel links point into both arrays.
#[derive(Debug, Default)]
pub(super) struct Scratch {
    pnls: Vec<pointnlink_t>,
    pnlps: Vec<usize>,
    pnll: usize,
    epnls: [pointnlink_t; 2],
    tris: Vec<triangle_t>,
    tril: usize,
    dq: deque_t,
}

impl Scratch {
    fn pnl(&self, pnlp: Pnl) -> &pointnlink_t {
        match pnlp {
            Pnl::Poly(i) => &self.pnls[i],
            Pnl::End(i) => &self.epnls[i],
        }
    }

    fn pnl_mut(&mut self, pnlp: Pnl) -> &mut pointnlink_t {
        match pnlp {
            Pnl::Poly(i) => &mut self.pnls[i],
            Pnl::End(i) => &mut self.epnls[i],
        }
    }

    fn dq_at(&self, index: usize) -> Pnl {
        self.dq.pnlps[index].expect("deque slots between its ends are filled")
    }

    fn dq_pp(&self, index: usize) -> Ppoint_t {
        self.pnl(self.dq_at(index)).pp
    }

    fn tri_pp(&self, pnl: usize) -> Ppoint_t {
        self.pnls[pnl].pp
    }
}

/// Finds the shortest path inside `polyp` from `eps[0]` to `eps[1]` and stores it in `output`.
///
/// # Panics
/// Where Smetana throws: when the source lies in no triangle of the polygon (`UNSUPPORTED` there).
pub fn Pshortestpath(
    zz: &mut PathplanContext,
    polyp: &Ppoly_t,
    eps: [Ppoint_t; 2],
    output: &mut Ppolyline_t,
) -> Result<(), PathplanError> {
    let zz = &mut zz.shortest;
    let pn = polyp.ps.len();

    growpnls(zz, pn);
    zz.pnll = 0;
    zz.tril = 0;
    growdq(zz, pn * 2);
    zz.dq.fpnlpi = zz.dq.pnlps.len() / 2;
    zz.dq.lpnlpi = zz.dq.fpnlpi - 1;

    loadpolygon(zz, &polyp.ps);

    triangulate(zz, zz.pnll)?;

    for trii in 0..zz.tril {
        for trij in trii + 1..zz.tril {
            connecttris(zz, trii, trij);
        }
    }

    let Some(ftrii) = (0..zz.tril).find(|&trii| pointintri(zz, trii, &eps[0])) else {
        unimplemented!("4ma3y8l4lmjcsw49kmsgknig6: source point not in any triangle")
    };
    let Some(ltrii) = (0..zz.tril).find(|&trii| pointintri(zz, trii, &eps[1])) else {
        return Err(PathplanError::DestinationOutside);
    };

    // mark the strip of triangles from eps[0] to eps[1]; if that fails, a straight line is better than failing.
    // If the endpoints are in the same triangle, use a single line.
    if !marktripath(zz, ftrii, ltrii) || ftrii == ltrii {
        output.ps.clear();
        output.ps.extend_from_slice(&eps);
        return Ok(());
    }

    zz.epnls = [
        pointnlink_t {
            pp: eps[0],
            link: None,
        },
        pointnlink_t {
            pp: eps[1],
            link: None,
        },
    ];
    add2dq(zz, DQ_FRONT, Pnl::End(0));
    zz.dq.apex = zz.dq.fpnlpi;
    let mut trii = Some(ftrii);
    while let Some(tri) = trii {
        zz.tris[tri].mark = 2;
        let trip = zz.tris[tri];

        let exit = (0..3).find(|&ei| trip.e[ei].rtp.is_some_and(|rtp| zz.tris[rtp].mark == 1));
        let (lpnlp, rpnlp) = if let Some(ei) = exit {
            let pnlp = trip.e[(ei + 1) % 3].pnl1p;
            if ccw(
                &zz.tri_pp(trip.e[ei].pnl0p),
                &zz.tri_pp(pnlp),
                &zz.tri_pp(trip.e[ei].pnl1p),
            ) == ISCCW
            {
                unimplemented!(
                    "2cii65lhw4wb8nyvjv702v7md: lpnlp = trip->e[ei].pnl1p, rpnlp = trip->e[ei].pnl0p"
                )
            }
            (Pnl::Poly(trip.e[ei].pnl0p), Pnl::Poly(trip.e[ei].pnl1p))
        } else if ccw(&eps[1], &zz.dq_pp(zz.dq.fpnlpi), &zz.dq_pp(zz.dq.lpnlpi)) == ISCCW {
            // in the last triangle
            (zz.dq_at(zz.dq.lpnlpi), Pnl::End(1))
        } else {
            (Pnl::End(1), zz.dq_at(zz.dq.lpnlpi))
        };

        if tri == ftrii {
            add2dq(zz, DQ_BACK, lpnlp);
            add2dq(zz, DQ_FRONT, rpnlp);
        } else if zz.dq.pnlps[zz.dq.fpnlpi] != Some(rpnlp)
            && zz.dq.pnlps[zz.dq.lpnlpi] != Some(rpnlp)
        {
            let splitindex = finddqsplit(zz, rpnlp);
            splitdq(zz, DQ_BACK, splitindex);
            add2dq(zz, DQ_FRONT, rpnlp);
            if splitindex > zz.dq.apex {
                zz.dq.apex = splitindex;
            }
        } else {
            let splitindex = finddqsplit(zz, lpnlp);
            splitdq(zz, DQ_FRONT, splitindex);
            add2dq(zz, DQ_BACK, lpnlp);
            if splitindex < zz.dq.apex {
                zz.dq.apex = splitindex;
            }
        }
        trii = trip
            .e
            .iter()
            .find_map(|e| e.rtp.filter(|&rtp| zz.tris[rtp].mark == 1));
    }

    output.ps.clear();
    let mut pnlp = Some(Pnl::End(1));
    while let Some(p) = pnlp {
        output.ps.push(zz.pnl(p).pp);
        pnlp = zz.pnl(p).link;
    }
    output.ps.reverse();
    Ok(())
}

/// Loads the polygon into `pnls` counterclockwise, without repeated consecutive points.
fn loadpolygon(zz: &mut Scratch, ps: &[Ppoint_t]) {
    let pn = ps.len();
    let mut minx = f64::INFINITY;
    let mut minpi = None;
    for (pi, p) in ps.iter().enumerate() {
        if minx > p.x {
            minx = p.x;
            minpi = Some(pi);
        }
    }
    let minpi = minpi.expect("the polygon has a leftmost point");
    let p2 = ps[minpi];
    let p1 = ps[if minpi == 0 { pn - 1 } else { minpi - 1 }];
    let p3 = ps[if minpi == pn - 1 { 0 } else { minpi + 1 }];
    if (p1.x == p2.x && p2.x == p3.x && p3.y > p2.y) || ccw(&p1, &p2, &p3) != ISCCW {
        for pi in (0..pn).rev() {
            if pi < pn - 1 && ps[pi].x == ps[pi + 1].x && ps[pi].y == ps[pi + 1].y {
                continue;
            }
            loadpnl(zz, ps[pi], pn);
        }
    } else {
        for pi in 0..pn {
            if pi > 0 && ps[pi].x == ps[pi - 1].x && ps[pi].y == ps[pi - 1].y {
                continue;
            }
            loadpnl(zz, ps[pi], pn);
        }
    }
}

fn loadpnl(zz: &mut Scratch, pp: Ppoint_t, pn: usize) {
    zz.pnls[zz.pnll] = pointnlink_t {
        pp,
        link: Some(Pnl::Poly(zz.pnll % pn)),
    };
    zz.pnlps[zz.pnll] = zz.pnll;
    zz.pnll += 1;
}

/// Clips ears off the polygon `pnlps[..pnln]` until a triangle is left. Smetana recurses after each ear; this loop
/// is that tail recursion.
fn triangulate(zz: &mut Scratch, mut pnln: usize) -> Result<(), PathplanError> {
    while pnln > 3 {
        let Some(pnli) = (0..pnln).find(|&pnli| isdiagonal(zz, pnli, (pnli + 2) % pnln, pnln))
        else {
            return Err(PathplanError::TriangulationFailed);
        };
        let pnlip1 = (pnli + 1) % pnln;
        let pnlip2 = (pnli + 2) % pnln;
        loadtriangle(zz, zz.pnlps[pnli], zz.pnlps[pnlip1], zz.pnlps[pnlip2]);
        zz.pnlps.copy_within(pnlip1 + 1..pnln, pnlip1);
        pnln -= 1;
    }
    loadtriangle(zz, zz.pnlps[0], zz.pnlps[1], zz.pnlps[2]);
    Ok(())
}

fn isdiagonal(zz: &Scratch, pnli: usize, pnlip2: usize, pnln: usize) -> bool {
    let pp = |i: usize| &zz.pnls[zz.pnlps[i]].pp;
    let pnlip1 = (pnli + 1) % pnln;
    let pnlim1 = (pnli + pnln - 1) % pnln;
    // If P[pnli] is a convex vertex [ pnli+1 left of (pnli-1,pnli) ].
    let res = if ccw(pp(pnlim1), pp(pnli), pp(pnlip1)) == ISCCW {
        ccw(pp(pnli), pp(pnlip2), pp(pnlim1)) == ISCCW
            && ccw(pp(pnlip2), pp(pnli), pp(pnlip1)) == ISCCW
    } else {
        // Assume (pnli - 1, pnli, pnli + 1) not collinear.
        ccw(pp(pnli), pp(pnlip2), pp(pnlip1)) == ISCW
    };
    if !res {
        return false;
    }
    for pnlj in 0..pnln {
        let pnljp1 = (pnlj + 1) % pnln;
        if !(pnlj == pnli || pnljp1 == pnli || pnlj == pnlip2 || pnljp1 == pnlip2)
            && intersects(pp(pnli), pp(pnlip2), pp(pnlj), pp(pnljp1))
        {
            return false;
        }
    }
    true
}

fn loadtriangle(zz: &mut Scratch, pnlap: usize, pnlbp: usize, pnlcp: usize) {
    if zz.tril >= zz.tris.len() {
        growtris(zz, zz.tris.len() + 20);
    }
    let edge = |pnl0p, pnl1p| tedge_t {
        pnl0p,
        pnl1p,
        rtp: None,
    };
    zz.tris[zz.tril] = triangle_t {
        mark: 0,
        e: [edge(pnlap, pnlbp), edge(pnlbp, pnlcp), edge(pnlcp, pnlap)],
    };
    zz.tril += 1;
}

/// Connects a pair of triangles at their common edge (if any).
fn connecttris(zz: &mut Scratch, tri1: usize, tri2: usize) {
    for ei in 0..3 {
        for ej in 0..3 {
            let e1 = zz.tris[tri1].e[ei];
            let e2 = zz.tris[tri2].e[ej];
            if (e1.pnl0p == e2.pnl0p && e1.pnl1p == e2.pnl1p)
                || (e1.pnl0p == e2.pnl1p && e1.pnl1p == e2.pnl0p)
            {
                zz.tris[tri1].e[ei].rtp = Some(tri2);
                zz.tris[tri2].e[ej].rtp = Some(tri1);
            }
        }
    }
}

fn marktripath(zz: &mut Scratch, trii: usize, trij: usize) -> bool {
    if zz.tris[trii].mark != 0 {
        return false;
    }
    zz.tris[trii].mark = 1;
    if trii == trij {
        return true;
    }
    for ei in 0..3 {
        if let Some(rtp) = zz.tris[trii].e[ei].rtp
            && marktripath(zz, rtp, trij)
        {
            return true;
        }
    }
    zz.tris[trii].mark = 0;
    false
}

fn add2dq(zz: &mut Scratch, side: i32, pnlp: Pnl) {
    let nonempty = zz.dq.lpnlpi >= zz.dq.fpnlpi;
    if side == DQ_FRONT {
        if nonempty {
            zz.pnl_mut(pnlp).link = zz.dq.pnlps[zz.dq.fpnlpi];
        }
        zz.dq.fpnlpi -= 1;
        zz.dq.pnlps[zz.dq.fpnlpi] = Some(pnlp);
    } else {
        if nonempty {
            zz.pnl_mut(pnlp).link = zz.dq.pnlps[zz.dq.lpnlpi];
        }
        zz.dq.lpnlpi += 1;
        zz.dq.pnlps[zz.dq.lpnlpi] = Some(pnlp);
    }
}

fn splitdq(zz: &mut Scratch, side: i32, index: usize) {
    if side == DQ_FRONT {
        zz.dq.lpnlpi = index;
    } else {
        zz.dq.fpnlpi = index;
    }
}

fn finddqsplit(zz: &Scratch, pnlp: Pnl) -> usize {
    let pp = zz.pnl(pnlp).pp;
    for index in zz.dq.fpnlpi..zz.dq.apex {
        if ccw(&zz.dq_pp(index + 1), &zz.dq_pp(index), &pp) == ISCCW {
            return index;
        }
    }
    let mut index = zz.dq.lpnlpi;
    while index > zz.dq.apex {
        if ccw(&zz.dq_pp(index - 1), &zz.dq_pp(index), &pp) == ISCW {
            return index;
        }
        index -= 1;
    }
    zz.dq.apex
}

fn ccw(p1p: &Ppoint_t, p2p: &Ppoint_t, p3p: &Ppoint_t) -> i32 {
    let d = ((p1p.y - p2p.y) * (p3p.x - p2p.x)) - ((p3p.y - p2p.y) * (p1p.x - p2p.x));
    if d > 0.0 {
        ISCCW
    } else if d < 0.0 {
        ISCW
    } else {
        ISON
    }
}

fn intersects(pap: &Ppoint_t, pbp: &Ppoint_t, pcp: &Ppoint_t, pdp: &Ppoint_t) -> bool {
    if ccw(pap, pbp, pcp) == ISON
        || ccw(pap, pbp, pdp) == ISON
        || ccw(pcp, pdp, pap) == ISON
        || ccw(pcp, pdp, pbp) == ISON
    {
        between(pap, pbp, pcp)
            || between(pap, pbp, pdp)
            || between(pcp, pdp, pap)
            || between(pcp, pdp, pbp)
    } else {
        let ccw1 = ccw(pap, pbp, pcp) == ISCCW;
        let ccw2 = ccw(pap, pbp, pdp) == ISCCW;
        let ccw3 = ccw(pcp, pdp, pap) == ISCCW;
        let ccw4 = ccw(pcp, pdp, pbp) == ISCCW;
        (ccw1 ^ ccw2) && (ccw3 ^ ccw4)
    }
}

fn between(pap: &Ppoint_t, pbp: &Ppoint_t, pcp: &Ppoint_t) -> bool {
    let p1 = Ppoint_t {
        x: pbp.x - pap.x,
        y: pbp.y - pap.y,
    };
    let p2 = Ppoint_t {
        x: pcp.x - pap.x,
        y: pcp.y - pap.y,
    };
    if ccw(pap, pbp, pcp) != ISON {
        return false;
    }
    (p2.x * p1.x + p2.y * p1.y >= 0.0) && (p2.x * p2.x + p2.y * p2.y <= p1.x * p1.x + p1.y * p1.y)
}

fn pointintri(zz: &Scratch, trii: usize, pp: &Ppoint_t) -> bool {
    let sum = zz.tris[trii]
        .e
        .iter()
        .filter(|e| ccw(&zz.tri_pp(e.pnl0p), &zz.tri_pp(e.pnl1p), pp) != ISCW)
        .count();
    sum == 3 || sum == 0
}

fn growpnls(zz: &mut Scratch, newpnln: usize) {
    if newpnln > zz.pnls.len() {
        zz.pnls.resize(newpnln, pointnlink_t::default());
        zz.pnlps.resize(newpnln, 0);
    }
}

fn growtris(zz: &mut Scratch, newtrin: usize) {
    if newtrin > zz.tris.len() {
        zz.tris.resize(newtrin, triangle_t::default());
    }
}

fn growdq(zz: &mut Scratch, newdqn: usize) {
    if newdqn > zz.dq.pnlps.len() {
        zz.dq.pnlps.resize(newdqn, None);
    }
}
