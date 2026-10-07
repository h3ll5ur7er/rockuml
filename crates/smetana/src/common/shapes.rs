//! `common/shapes.c`: node shapes. PlantUML uses `box` (the general polygon code), `ellipse` and `record`. This
//! part binds a node to its shape, sizes it (`poly_init`, `record_init`) and resolves edge ports
//! (`poly_port`, `record_port`).

use crate::cgraph::obj::agraphof;
use crate::common::geom::cwrotatepf;
use crate::common::labels::make_label;
use crate::common::utils::{agget_text, late_double, late_int, late_string, mapbool};
use crate::core::Globals;
use crate::core::consts::{BOTTOM, GAP, LEFT, LT_HTML, LT_NONE, M_PI, RIGHT, TOP};
use crate::core::ids::{FieldId, NodeId, ShapeDescId};
use crate::core::jmath::{POINTS, PS2INCH, ROUND, atan2, cos, hypot, max, sin};
use crate::h::{
    Center, SHAPE_INFO, boxf, field_t, pointf, pointfof, polygon_t, port, shape_functions,
};

/// `poly_init` option: the node keeps its given size whatever the label.
const FIXEDSHAPE: i32 = 1 << 11;
const SQRT2: f64 = std::f64::consts::SQRT_2;

/// `EN_shape_kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EN_shape_kind {
    SH_UNSET,
    SH_POLY,
    SH_RECORD,
}

/// `shapeOf`: which shape code a node uses.
pub fn shapeOf(zz: &Globals, n: NodeId) -> EN_shape_kind {
    match zz.nd(n).shape {
        None => EN_shape_kind::SH_UNSET,
        Some(sh) => match zz.Shapes[sh].fns {
            shape_functions::poly_fns => EN_shape_kind::SH_POLY,
            shape_functions::record_fns => EN_shape_kind::SH_RECORD,
        },
    }
}

/// `bind_shape`: the shape called `name`. Smetana has no user shapes and no `shapefile`.
pub fn bind_shape(zz: &Globals, name: &str) -> ShapeDescId {
    zz.Shapes
        .ids()
        .find(|&sh| zz.Shapes[sh].name == name)
        .unwrap_or_else(|| unimplemented!("user shape {name:?}"))
}

/// `ND_shape(n)->fns->initfn(n)`.
pub fn initfn(zz: &mut Globals, n: NodeId) {
    match zz.Shapes[zz.nd(n).shape.expect("node without shape")].fns {
        shape_functions::poly_fns => poly_init(zz, n),
        shape_functions::record_fns => record_init(zz, n),
    }
}

/// `ND_shape(n)->fns->portfn(n, portname, compass)`.
pub fn portfn(zz: &mut Globals, n: NodeId, portname: &str, compass: Option<&str>) -> port {
    match zz.Shapes[zz.nd(n).shape.expect("node without shape")].fns {
        shape_functions::poly_fns => poly_port(zz, n, portname, compass),
        shape_functions::record_fns => record_port(zz, n, portname, compass),
    }
}

/// `PAD`: the minimal whitespace around a label.
pub(crate) fn PAD(d: &mut pointf) {
    d.x += f64::from(4 * GAP);
    d.y += f64::from(2 * GAP);
}

fn SQR(a: f64) -> f64 {
    a * a
}

fn RADIANS(deg: f64) -> f64 {
    deg / 180.0 * M_PI
}

/// `poly_init`: sizes a polygon (or ellipse) node around its label and computes its vertices.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
pub fn poly_init(zz: &mut Globals, n: NodeId) {
    let shape = zz.nd(n).shape.expect("node without shape");
    let polygon = zz.Shapes[shape].polygon.expect("polygon shape");
    let regular = polygon.regular;
    let mut peripheries = polygon.peripheries;
    let mut sides = polygon.sides;
    let mut orientation = polygon.orientation;
    let skew = polygon.skew;
    let distortion = polygon.distortion;
    let regular = regular | mapbool(agget_text(zz, n, "regular").as_deref());

    // All calculations in floating-point points.
    if regular {
        unimplemented!("regular polygons");
    }
    let mut width = f64::from(POINTS(zz.nd(n).width));
    let mut height = f64::from(POINTS(zz.nd(n).height));

    peripheries = late_int(zz, n, zz.N_peripheries, peripheries, 0);
    orientation += late_double(zz, n, zz.N_orientation, 0.0, -360.0);
    if sides == 0 {
        unimplemented!("polygon shapes with user sides");
    }

    let label = zz.nd(n).label.expect("node label");
    let mut dimen = zz.textlabels[label].dimen;
    if ROUND(dimen.x.abs()) != 0 || ROUND(dimen.y.abs()) != 0 {
        if agget_text(zz, n, "margin").is_some() {
            unimplemented!("node margin");
        }
        PAD(&mut dimen);
    }
    let spacex = dimen.x - zz.textlabels[label].dimen.x;

    let root = agraphof(zz, n);
    if zz.gd(root).drawing.expect("GD_drawing").quantum > 0.0 {
        unimplemented!("quantum");
    }

    if zz.Shapes[shape].usershape {
        unimplemented!("user shapes");
    } else if agget_text(zz, n, "image").is_some_and(|s| !s.is_empty()) {
        unimplemented!("node images");
    }
    let imagesize = pointf::default();

    // The node's box starts at the label's size.
    let mut bb = pointf {
        x: max(dimen.x, imagesize.x),
        y: max(dimen.y, imagesize.y),
    };

    // Ellipses cannot be distorted or skewed: they become polygons with many sides.
    if sides <= 2 && (distortion != 0.0 || skew != 0.0) {
        sides = 120;
    }

    // Extra sizing depends on whether the label is centered vertically.
    let valign = match agget_text(zz, n, "labelloc").and_then(|p| p.chars().next()) {
        Some(c @ ('t' | 'b')) => c as i32,
        _ => 'c' as i32,
    };
    zz.textlabels[label].valign = valign;

    let isBox = sides == 4 && (ROUND(orientation) % 90) == 0 && distortion == 0.0 && skew == 0.0;
    if isBox {
        // For regular boxes the fit should be exact.
    } else if polygon.vertices.is_some() {
        unimplemented!("polygons with generated vertices");
    } else {
        // For all other shapes, the smallest ellipse containing bb centered on the origin, padded for that.
        let temp = bb.y * SQRT2;
        if height > temp && valign == 'c' as i32 {
            // Height to spare and a centered label: just pad x in proportion to the spare height.
            bb.x *= (1. / (1. - SQR(bb.y / height))).sqrt();
        } else {
            bb.x *= SQRT2;
            bb.y = temp;
        }
        if sides > 2 {
            let temp = cos(M_PI / f64::from(sides));
            bb.x /= temp;
            bb.y /= temp;
        }
    }

    // bb is now the minimum size of node that can hold the label.
    let min_bb = bb;

    // Increase the node size to width/height if needed.
    let mut poly = polygon_t::default();
    let fxd = late_string(zz, n, zz.N_fixed, Some("false")).expect("fixedsize");
    if fxd == "shape" {
        bb.x = width;
        bb.y = height;
        poly.option |= FIXEDSHAPE;
    } else if mapbool(Some(&fxd)) {
        bb.x = width;
        bb.y = height;
    } else {
        width = max(width, bb.x);
        bb.x = width;
        height = max(height, bb.y);
        bb.y = height;
    }

    // The space available for the label, which gives the justification borders.
    let nojustify = late_string(zz, n, zz.N_nojustify, Some("false"));
    let lp = &mut zz.textlabels[label];
    if mapbool(nojustify.as_deref()) {
        lp.space.x = dimen.x - spacex;
    } else if isBox {
        lp.space.x = max(dimen.x, bb.x) - spacex;
    } else if dimen.y < bb.y {
        let temp = bb.x * (1.0 - SQR(dimen.y) / SQR(bb.y)).sqrt();
        lp.space.x = max(dimen.x, temp) - spacex;
    } else {
        lp.space.x = dimen.x - spacex;
    }

    if (poly.option & FIXEDSHAPE) == 0 {
        let mut temp = bb.y - min_bb.y;
        if dimen.y < imagesize.y {
            temp += imagesize.y - dimen.y;
        }
        lp.space.y = dimen.y + temp;
    }

    let outp = if peripheries < 1 { 1 } else { peripheries };
    let vertices;
    if sides < 3 {
        // Ellipses.
        sides = 2;
        vertices = zz.pointfs.ALLOC(outp * sides);
        let P = pointf {
            x: bb.x / 2.,
            y: bb.y / 2.,
        };
        zz.pointfs.set(vertices, 0, pointf { x: -P.x, y: -P.y });
        zz.pointfs.set(vertices, 1, P);
        if peripheries > 1 {
            unimplemented!("ellipse peripheries");
        }
    } else {
        vertices = zz.pointfs.ALLOC(outp * sides);
        let sectorangle = 2. * M_PI / f64::from(sides);
        let sidelength = sin(sectorangle / 2.);
        let skewdist = hypot(distortion.abs() + skew.abs(), 1.);
        let gdistortion = distortion * SQRT2 / cos(sectorangle / 2.);
        let gskew = skew / 2.;
        let mut angle = (sectorangle - M_PI) / 2.;
        let mut R = pointf {
            x: 0.5 * cos(angle),
            y: 0.5 * sin(angle),
        };
        let (mut xmax, mut ymax) = (0., 0.);
        angle += (M_PI - sectorangle) / 2.;
        for i in 0..sides {
            // The next regular vertex.
            angle += sectorangle;
            R.x += sidelength * cos(angle);
            R.y += sidelength * sin(angle);

            // Distort and skew.
            let mut P = pointf {
                x: R.x * (skewdist + R.y * gdistortion) + R.y * gskew,
                y: R.y,
            };

            // Orient P.
            let alpha = RADIANS(orientation) + atan2(P.y, P.x);
            let (sinx, cosx) = (sin(alpha), cos(alpha));
            P.x = hypot(P.x, P.y);
            P.y = P.x;
            P.x *= cosx;
            P.y *= sinx;

            // Scale for the label.
            P.x *= bb.x;
            P.y *= bb.y;

            // The bounding box.
            xmax = max(P.x.abs(), xmax);
            ymax = max(P.y.abs(), ymax);

            zz.pointfs.set(vertices, i, P);
            if isBox {
                // Enforce the exact symmetry of a box.
                zz.pointfs.set(vertices, 1, pointf { x: -P.x, y: P.y });
                zz.pointfs.set(vertices, 2, pointf { x: -P.x, y: -P.y });
                zz.pointfs.set(vertices, 3, pointf { x: P.x, y: -P.y });
                break;
            }
        }

        // Apply the minimum dimensions.
        xmax *= 2.;
        ymax *= 2.;
        bb.x = max(width, xmax);
        bb.y = max(height, ymax);
        let scalex = bb.x / xmax;
        let scaley = bb.y / ymax;

        for i in 0..sides {
            let mut P = zz.pointfs.get(vertices, i);
            P.x *= scalex;
            P.y *= scaley;
            zz.pointfs.set(vertices, i, P);
        }
        if peripheries > 1 {
            unimplemented!("polygon peripheries");
        }
    }
    poly.regular = regular;
    poly.peripheries = peripheries;
    poly.sides = sides;
    poly.orientation = orientation;
    poly.skew = skew;
    poly.distortion = distortion;
    poly.vertices = Some(vertices);

    if (poly.option & FIXEDSHAPE) != 0 {
        unimplemented!("fixedsize=shape");
    }
    zz.nd_mut(n).width = PS2INCH(bb.x);
    zz.nd_mut(n).height = PS2INCH(bb.y);
    let poly = zz.polygons.push(poly);
    zz.nd_mut(n).shape_info = Some(SHAPE_INFO::Polygon(poly));
}

/// `invflip_side`: maps a side back from the rotated layout. PlantUML only lays out top to bottom here.
fn invflip_side(side: i32, rankdir: i32) -> i32 {
    match rankdir {
        0 => side,
        _ => unimplemented!("invflip_side for rankdir {rankdir}"),
    }
}

/// `invflip_angle`: maps an angle back from the rotated layout.
fn invflip_angle(angle: f64, rankdir: i32) -> f64 {
    match rankdir {
        0 => angle,
        _ => unimplemented!("invflip_angle for rankdir {rankdir}"),
    }
}

/// `compassPort`: fills `pp` for the compass point `compass` on box `bp` (the node's box if `None`). Returns
/// whether the compass point was not recognized. Smetana never clips to the shape here (`ictxt` is NULL).
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
pub(crate) fn compassPort(
    zz: &Globals,
    n: NodeId,
    bp: Option<boxf>,
    pp: &mut port,
    compass: &str,
    sides: i32,
) -> bool {
    let mut b = boxf::default();
    let mut p = pointf::default();
    let mut rv = 0;
    let mut theta = 0.0;
    let mut constrain = false;
    let mut dyna = false;
    let mut side = 0;
    let mut clip = true;
    let mut defined;
    let g = agraphof(zz, n);
    if let Some(bp) = bp {
        b = bp;
        p = pointfof((b.LL.x + b.UR.x) / 2., (b.LL.y + b.UR.y) / 2.);
        defined = true;
    } else {
        if zz.gd(g).GD_flip() {
            unimplemented!("compassPort on a flipped graph");
        }
        b.UR.y = zz.nd(n).ht / 2.;
        b.LL.y = -b.UR.y;
        b.UR.x = zz.nd(n).lw;
        b.LL.x = -b.UR.x;
        defined = false;
    }
    let ctr = p;
    let mut compass = compass.chars();
    if let Some(first) = compass.next() {
        let rest = compass.next();
        match first {
            'e' => {
                if rest.is_some() {
                    unimplemented!("compass point e...");
                }
                p.x = b.UR.x;
                theta = 0.0;
                constrain = true;
                defined = true;
                clip = false;
                side = sides & RIGHT;
            }
            's' => {
                p.y = b.LL.y;
                constrain = true;
                clip = false;
                match rest {
                    None => {
                        theta = -M_PI * 0.5;
                        defined = true;
                        p.x = ctr.x;
                        side = sides & BOTTOM;
                    }
                    Some(c) => unimplemented!("compass point s{c}"),
                }
            }
            'w' => {
                if rest.is_some() {
                    rv = 1;
                } else {
                    p.x = b.LL.x;
                    theta = M_PI;
                    constrain = true;
                    defined = true;
                    clip = false;
                    side = sides & LEFT;
                }
            }
            'n' => {
                p.y = b.UR.y;
                constrain = true;
                clip = false;
                match rest {
                    None => {
                        defined = true;
                        theta = M_PI * 0.5;
                        p.x = ctr.x;
                        side = sides & TOP;
                    }
                    Some(c) => unimplemented!("compass point n{c}"),
                }
            }
            '_' => {
                dyna = true;
                side = sides;
            }
            'c' => unimplemented!("compass point c"),
            _ => rv = 1,
        }
    }
    let rankdir = zz.gd(g).GD_rankdir();
    p = cwrotatepf(p, 90 * rankdir);
    pp.side = if dyna {
        side
    } else {
        invflip_side(side, rankdir)
    };
    pp.bp = bp;
    pp.p = pointf {
        x: f64::from(ROUND(p.x)),
        y: f64::from(ROUND(p.y)),
    };
    pp.theta = invflip_angle(theta, rankdir);
    if p.x == 0. && p.y == 0. {
        pp.order = 256 / 2;
    } else {
        // The angle with 0 at the north pole, increasing counter-clockwise.
        let mut angle = atan2(p.y, p.x) + 1.5 * M_PI;
        if angle >= 2. * M_PI {
            angle -= 2. * M_PI;
        }
        pp.order = ((256. * angle) / (2. * M_PI)) as i32;
    }
    pp.constrained = constrain;
    pp.defined = defined;
    pp.clip = clip;
    pp.dyna = dyna;
    rv != 0
}

/// `poly_port`: the port of a polygon node, which can only be a compass point.
/// Like Graphviz, it reads the port name as the compass point and ignores `compass`.
fn poly_port(zz: &mut Globals, n: NodeId, portname: &str, _compass: Option<&str>) -> port {
    if portname.is_empty() {
        return Center;
    }
    let sides = BOTTOM | RIGHT | TOP | LEFT;
    let label = zz.nd(n).label.expect("node label");
    if zz.textlabels[label].html {
        unimplemented!("HTML ports");
    }
    let shape = zz.nd(n).shape.expect("node without shape");
    if zz.Shapes[shape].name != "box" {
        unimplemented!("ports on non-box polygons");
    }
    let mut rv = port::default();
    // An unrecognized port only makes Graphviz print a warning.
    compassPort(zz, n, None, &mut rv, portname, sides);
    rv
}

/// Record label parsing flags.
const HASTEXT: i32 = 1;
const HASPORT: i32 = 2;
const HASTABLE: i32 = 4;
const INTEXT: i32 = 8;
const INPORT: i32 = 16;

/// The state `parse_reclbl` shares across its recursion: the label being read (`reclblp`, a former static) and
/// the scratch buffer each field's text and port name are assembled in. Both are UTF-16, as Java's strings.
struct RecordLabel {
    reclbl: Vec<u16>,
    reclblp: usize,
    text: Vec<u16>,
}

impl RecordLabel {
    fn at(&self) -> u16 {
        self.reclbl[self.reclblp]
    }

    /// The text buffer's content up to its first NUL.
    fn content(&self) -> String {
        let end = self
            .text
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.text.len());
        String::from_utf16_lossy(&self.text[..end])
    }
}

fn parse_error() -> ! {
    unimplemented!("record label parse errors")
}

const fn u(c: char) -> u16 {
    c as u16
}

/// `parse_reclbl`: parses one level of a record label into a field and its subfields.
#[allow(clippy::too_many_lines, reason = "one Graphviz function")]
fn parse_reclbl(
    zz: &mut Globals,
    n: NodeId,
    LR: bool,
    flag: bool,
    rl: &mut RecordLabel,
) -> FieldId {
    let lbl = zz.nd(n).label.expect("node label");
    let html = zz.textlabels[lbl].html;

    // Count the fields at this level.
    let mut maxf = 1;
    let mut cnt = 0;
    let mut sp = rl.reclblp;
    while rl.reclbl[sp] != 0 {
        if rl.reclbl[sp] == u('\\') {
            sp += 1;
            if [u('{'), u('}'), u('|'), u('\\')].contains(&rl.reclbl[sp]) {
                unimplemented!("escaped characters in record labels");
            }
        }
        let c = rl.reclbl[sp];
        if c == u('{') {
            cnt += 1;
        } else if c == u('}') {
            cnt -= 1;
        } else if c == u('|') && cnt == 0 {
            maxf += 1;
        }
        if cnt < 0 {
            break;
        }
        sp += 1;
    }
    let fld = zz.field_lists.ALLOC(maxf);
    let mut rv = field_t {
        fld: Some(fld),
        LR,
        ..field_t::default()
    };
    let mut fp: Option<FieldId> = None;
    let mut mode = 0;
    let mut fi = 0;
    let (mut tsp, mut hstsp) = (0usize, 0usize);
    let (mut psp, mut hspsp) = (0usize, 0usize);
    let mut tmpport: Option<String> = None;
    let mut wflag = true;
    let ishardspace = false;
    while wflag {
        let c = rl.at();
        if c < u(' ') && c != 0 {
            // Control characters are ignored.
            rl.reclblp += 1;
            continue;
        }
        match c {
            0x3c /* '<' */ => {
                if (mode & (HASTABLE | HASPORT)) != 0 {
                    parse_error();
                }
                if html {
                    unimplemented!("HTML record labels");
                }
                mode |= HASPORT | INPORT;
                rl.reclblp += 1;
                psp = 0;
                hspsp = 0;
            }
            0x3e /* '>' */ => {
                if html {
                    unimplemented!("HTML record labels");
                }
                if (mode & INPORT) == 0 {
                    parse_error();
                }
                if psp > 1 && psp - 1 != hspsp && rl.text[psp - 1] == u(' ') {
                    unimplemented!("port names ending with a space");
                }
                rl.text[psp] = 0;
                tmpport = Some(rl.content());
                mode &= !INPORT;
                rl.reclblp += 1;
            }
            0x7b /* '{' */ => {
                rl.reclblp += 1;
                if mode != 0 || rl.at() == 0 {
                    parse_error();
                }
                mode = HASTABLE;
                let sub = parse_reclbl(zz, n, !LR, false, rl);
                zz.field_lists.set(fld, fi, Some(sub));
                fi += 1;
            }
            0x7d | 0x7c | 0 /* '}', '|', NUL */ => {
                if (c == 0 && !flag) || (mode & INPORT) != 0 {
                    parse_error();
                }
                if (mode & HASTABLE) == 0 {
                    let new = zz.fields.push(field_t::default());
                    zz.field_lists.set(fld, fi, Some(new));
                    fi += 1;
                    fp = Some(new);
                }
                if let Some(port) = tmpport.take() {
                    zz.fields[fp.expect("field")].id = Some(port);
                }
                if (mode & (HASTEXT | HASTABLE)) == 0 {
                    mode |= HASTEXT;
                    rl.text[tsp] = u(' ');
                    tsp += 1;
                }
                if (mode & HASTEXT) != 0 {
                    if tsp > 1 && tsp - 1 != hstsp && rl.text[tsp - 1] == u(' ') {
                        tsp -= 1;
                    }
                    rl.text[tsp] = 0;
                    let text = rl.content();
                    let label = &zz.textlabels[lbl];
                    let (fontsize, fontname, fontcolor) =
                        (label.fontsize, label.fontname.clone(), label.fontcolor.clone());
                    let kind = if html { LT_HTML } else { LT_NONE };
                    let lp = make_label(zz, n.into(), &text, kind, fontsize, &fontname, &fontcolor);
                    let f = &mut zz.fields[fp.expect("field")];
                    f.lp = Some(lp);
                    f.LR = true;
                    tsp = 0;
                    hstsp = 0;
                }
                if rl.at() != 0 {
                    if rl.at() == u('}') {
                        rl.reclblp += 1;
                        rv.n_flds = fi;
                        return zz.fields.push(rv);
                    }
                    mode = 0;
                    rl.reclblp += 1;
                } else {
                    wflag = false;
                }
            }
            0x5c /* '\\' */ => unimplemented!("backslashes in record labels"),
            _ => {
                if (mode & HASTABLE) != 0 && c != u(' ') {
                    parse_error();
                }
                if (mode & (INTEXT | INPORT)) == 0 && c != u(' ') {
                    mode |= INTEXT | HASTEXT;
                }
                if (mode & INTEXT) != 0 {
                    if c != u(' ') || ishardspace || rl.text[tsp - 1] != u(' ') || html {
                        rl.text[tsp] = c;
                        tsp += 1;
                    }
                } else if (mode & INPORT) != 0
                    && !(c == u(' ') && !ishardspace && (psp == 0 || rl.text[psp - 1] == u(' ')))
                {
                    rl.text[psp] = c;
                    psp += 1;
                }
                rl.reclblp += 1;
                if (rl.at() & 128) != 0 {
                    unimplemented!("record label characters with bit 7 set");
                }
            }
        }
    }
    rv.n_flds = fi;
    zz.fields.push(rv)
}

/// The `i`-th subfield of `f`.
fn subfield(zz: &Globals, f: FieldId, i: i32) -> FieldId {
    let fld = zz.fields[f].fld.expect("subfields");
    zz.field_lists.get(fld, i).expect("subfield")
}

/// `size_reclbl`: the minimal size of a field: its label's, or the sum of its subfields' along its direction.
fn size_reclbl(zz: &mut Globals, n: NodeId, f: FieldId) -> pointf {
    let mut d = pointf::default();
    if let Some(lp) = zz.fields[f].lp {
        let dimen = zz.textlabels[lp].dimen;
        // Smetana adds no margin around record labels (PlantUML patch of 18/03/2023).
        if (dimen.x > 0.0 || dimen.y > 0.0) && agget_text(zz, n, "margin").is_some() {
            unimplemented!("record margin");
        }
        d = dimen;
    } else {
        for i in 0..zz.fields[f].n_flds {
            let sub = subfield(zz, f, i);
            let d0 = size_reclbl(zz, n, sub);
            if zz.fields[f].LR {
                d.x += d0.x;
                d.y = max(d.y, d0.y);
            } else {
                d.y += d0.y;
                d.x = max(d.x, d0.x);
            }
        }
    }
    zz.fields[f].size = d;
    d
}

/// `resize_reclbl`: grows a field to `sz`, sharing the extra space among its subfields.
fn resize_reclbl(zz: &mut Globals, f: FieldId, sz: pointf, nojustify_p: bool) {
    let d = pointf {
        x: sz.x - zz.fields[f].size.x,
        y: sz.y - zz.fields[f].size.y,
    };
    zz.fields[f].size = sz;

    // Adjust the text area.
    if let Some(lp) = zz.fields[f].lp
        && !nojustify_p
    {
        zz.textlabels[lp].space.x += d.x;
        zz.textlabels[lp].space.y += d.y;
    }

    // Adjust the children.
    let (n_flds, LR) = (zz.fields[f].n_flds, zz.fields[f].LR);
    if n_flds != 0 {
        let inc = if LR {
            d.x / f64::from(n_flds)
        } else {
            d.y / f64::from(n_flds)
        };
        for i in 0..n_flds {
            let sf = subfield(zz, f, i);
            let amt = ((f64::from(i + 1) * inc) as i32) - ((f64::from(i) * inc) as i32);
            let size = zz.fields[sf].size;
            let newsz = if LR {
                pointfof(size.x + f64::from(amt), sz.y)
            } else {
                pointfof(sz.x, size.y + f64::from(amt))
            };
            resize_reclbl(zz, sf, newsz, nojustify_p);
        }
    }
}

/// `pos_reclbl`: places a field's box with its upper left corner at `ul`, and its subfields inside.
fn pos_reclbl(zz: &mut Globals, f: FieldId, mut ul: pointf, sides: i32) {
    let field = &mut zz.fields[f];
    field.sides = sides;
    field.b.LL = pointfof(ul.x, ul.y - field.size.y);
    field.b.UR = pointfof(ul.x + field.size.x, ul.y);
    let (last, LR) = (field.n_flds - 1, field.LR);
    for i in 0..=last {
        let mask = if sides == 0 {
            0
        } else if LR {
            if i == 0 {
                if i == last {
                    TOP | BOTTOM | RIGHT | LEFT
                } else {
                    TOP | BOTTOM | LEFT
                }
            } else if i == last {
                TOP | BOTTOM | RIGHT
            } else {
                TOP | BOTTOM
            }
        } else if i == 0 {
            if i == last {
                unimplemented!("a single vertical subfield");
            }
            TOP | RIGHT | LEFT
        } else if i == last {
            LEFT | BOTTOM | RIGHT
        } else {
            LEFT | RIGHT
        };
        let sub = subfield(zz, f, i);
        pos_reclbl(zz, sub, ul, sides & mask);
        if LR {
            ul.x += zz.fields[sub].size.x;
        } else {
            ul.y -= zz.fields[sub].size.y;
        }
    }
}

/// `record_init`: parses a record node's label into fields and sizes the node around them.
pub fn record_init(zz: &mut Globals, n: NodeId) {
    let sides = BOTTOM | RIGHT | TOP | LEFT;
    // Always use rankdir to determine how records are laid out.
    let root = agraphof(zz, n);
    let flip = !zz.gd(root).GD_realflip();
    let label = zz.nd(n).label.expect("node label");
    let mut reclbl: Vec<u16> = zz.textlabels[label].text.encode_utf16().collect();
    // An empty label is parsed into a space, so the buffer needs at least two characters.
    let len = reclbl.len().max(1);
    reclbl.push(0);
    let mut rl = RecordLabel {
        reclbl,
        reclblp: 0,
        text: vec![0; len + 1],
    };
    let info = parse_reclbl(zz, n, flip, true, &mut rl);
    size_reclbl(zz, n, info);
    let mut sz = pointf {
        x: f64::from(POINTS(zz.nd(n).width)),
        y: f64::from(POINTS(zz.nd(n).height)),
    };
    let fixed = late_string(zz, n, zz.N_fixed, Some("false"));
    if mapbool(fixed.as_deref()) {
        unimplemented!("fixedsize records");
    }
    let size = zz.fields[info].size;
    sz.x = max(size.x, sz.x);
    sz.y = max(size.y, sz.y);
    let nojustify = late_string(zz, n, zz.N_nojustify, Some("false"));
    resize_reclbl(zz, info, sz, mapbool(nojustify.as_deref()));
    let ul = pointfof(-sz.x / 2., sz.y / 2.);
    pos_reclbl(zz, info, ul, sides);
    let size = zz.fields[info].size;
    zz.nd_mut(n).width = PS2INCH(size.x);
    // Kluge: +1 fixes a rounding difference between layout and rendering that could give -1 coordinates.
    zz.nd_mut(n).height = PS2INCH(size.y + 1.);
    zz.nd_mut(n).shape_info = Some(SHAPE_INFO::Field(info));
}

/// `map_rec_port`: the field (or subfield) whose port is named `str`.
fn map_rec_port(zz: &Globals, f: FieldId, str: &str) -> Option<FieldId> {
    if zz.fields[f].id.as_deref() == Some(str) {
        return Some(f);
    }
    (0..zz.fields[f].n_flds).find_map(|sub| map_rec_port(zz, subfield(zz, f, sub), str))
}

/// `record_port`: the port of a record node: one of its fields, or a compass point of the whole node.
fn record_port(zz: &mut Globals, n: NodeId, portname: &str, compass: Option<&str>) -> port {
    if portname.is_empty() {
        return Center;
    }
    let sides = BOTTOM | RIGHT | TOP | LEFT;
    let compass = compass.unwrap_or("_");
    let Some(SHAPE_INFO::Field(f)) = zz.nd(n).shape_info else {
        panic!("record without fields")
    };
    let mut rv = port::default();
    if let Some(subf) = map_rec_port(zz, f, portname) {
        let (b, subsides) = (zz.fields[subf].b, zz.fields[subf].sides);
        if compassPort(zz, n, Some(b), &mut rv, compass, subsides) {
            unimplemented!("unrecognized compass point {compass:?}");
        }
    } else {
        // An unrecognized port only makes Graphviz print a warning.
        let b = zz.fields[f].b;
        compassPort(zz, n, Some(b), &mut rv, portname, sides);
    }
    rv
}
