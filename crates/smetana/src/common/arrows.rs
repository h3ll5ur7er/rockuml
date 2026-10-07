//! `arrows.c`: which arrowheads an edge has, how long they are, and clipping splines to leave room for them.
//! Smetana knows two arrow names, `normal` and `none`; drawing arrows is PlantUML's business.

use crate::cgraph::attr::agxget;
use crate::common::geom::DIST2;
use crate::common::splines::bezier_clip;
use crate::common::utils::late_double;
use crate::core::Globals;
use crate::core::ids::{EdgeId, SymId};
use crate::h::{bezier, inside_t, pointf};

pub const ARR_TYPE_NONE: i32 = 0;
pub const ARR_TYPE_NORM: i32 = 1;
pub const ARR_TYPE_CROW: i32 = 2;
pub const ARR_TYPE_TEE: i32 = 3;
pub const ARR_TYPE_BOX: i32 = 4;
pub const ARR_TYPE_DIAMOND: i32 = 5;
pub const ARR_TYPE_DOT: i32 = 6;
pub const ARR_TYPE_CURVE: i32 = 7;
pub const ARR_TYPE_GAP: i32 = 8;

const BITS_PER_ARROW: i32 = 8;
const BITS_PER_ARROW_TYPE: i32 = 4;
const NUMB_OF_ARROW_HEADS: i32 = 4;

struct arrowname_t {
    name: &'static str,
    type_: i32,
}

struct arrowtype_t {
    type_: i32,
    lenfact: f64,
}

/// Smetana's tables (`Globals.Arrowsynonyms`...): no synonyms or modifiers, two names.
const Arrowsynonyms: [arrowname_t; 0] = [];
const Arrowmods: [arrowname_t; 0] = [];
const Arrownames: [arrowname_t; 2] = [
    arrowname_t {
        name: "normal",
        type_: ARR_TYPE_NORM,
    },
    arrowname_t {
        name: "none",
        type_: ARR_TYPE_GAP,
    },
];
const Arrowtypes: [arrowtype_t; 8] = [
    arrowtype_t {
        type_: ARR_TYPE_NORM,
        lenfact: 1.0,
    },
    arrowtype_t {
        type_: ARR_TYPE_CROW,
        lenfact: 1.0,
    },
    arrowtype_t {
        type_: ARR_TYPE_TEE,
        lenfact: 0.5,
    },
    arrowtype_t {
        type_: ARR_TYPE_BOX,
        lenfact: 1.0,
    },
    arrowtype_t {
        type_: ARR_TYPE_DIAMOND,
        lenfact: 1.2,
    },
    arrowtype_t {
        type_: ARR_TYPE_DOT,
        lenfact: 0.8,
    },
    arrowtype_t {
        type_: ARR_TYPE_CURVE,
        lenfact: 1.0,
    },
    arrowtype_t {
        type_: ARR_TYPE_GAP,
        lenfact: 0.5,
    },
];

/// `arrow_match_name_frag`: ORs the type of the table name `name` starts with into `flag`; returns the rest.
fn arrow_match_name_frag<'a>(name: &'a str, arrownames: &[arrowname_t], flag: &mut i32) -> &'a str {
    for arrowname in arrownames {
        if let Some(rest) = name.strip_prefix(arrowname.name) {
            *flag |= arrowname.type_;
            return rest;
        }
    }
    name
}

/// `arrow_match_shape`: matches one arrow shape at the start of `name`; returns the rest.
fn arrow_match_shape<'a>(name: &'a str, flag: &mut i32) -> &'a str {
    let mut f = 0;
    // Rests are suffixes of `name`, so equal lengths mean the same position (C compares pointers).
    let mut rest = arrow_match_name_frag(name, &Arrowsynonyms, &mut f);
    if rest.len() == name.len() {
        loop {
            let next = rest;
            rest = arrow_match_name_frag(next, &Arrowmods, &mut f);
            if next.len() == rest.len() {
                break;
            }
        }
        rest = arrow_match_name_frag(rest, &Arrownames, &mut f);
    }
    if f != 0 && (f & ((1 << BITS_PER_ARROW_TYPE) - 1)) == 0 {
        unimplemented!("2mly07gipiope02mgflzcie3e: arrow modifier without a shape");
    }
    *flag |= f;
    rest
}

/// `arrow_match_name`: the flags of an `arrowhead`/`arrowtail` value. Smetana never advances to the next head,
/// so every shape lands in the first one, and a lone `none` means no arrow at all. An unknown name (Smetana
/// prints a warning) leaves no arrow.
fn arrow_match_name(name: &str, flag: &mut i32) {
    let mut rest = name;
    *flag = 0;
    while !rest.is_empty() {
        let mut f = ARR_TYPE_NONE;
        let next = rest;
        rest = arrow_match_shape(next, &mut f);
        if f == ARR_TYPE_NONE {
            return;
        }
        if f == ARR_TYPE_GAP && rest.is_empty() {
            f = ARR_TYPE_NONE;
        }
        if f != ARR_TYPE_NONE {
            *flag |= f;
        }
    }
}

fn attribute(zz: &mut Globals, e: EdgeId, sym: Option<SymId>) -> Option<String> {
    let value = agxget(zz, e, sym?).expect("declared attribute without value");
    Some(zz.agstr(value).to_string()).filter(|v| !v.is_empty())
}

/// `arrow_flags`: the arrowheads at the start and end of `e`. Smetana ignores `dir` and `agisdirected`: both ends
/// start as `normal`.
pub fn arrow_flags(zz: &mut Globals, e: EdgeId, sflag: &mut i32, eflag: &mut i32) {
    *sflag = ARR_TYPE_NORM;
    *eflag = ARR_TYPE_NORM;
    if attribute(zz, e, zz.E_dir).is_some() {
        unimplemented!("em7x45v09orjeey5u06gf9b4s: dir");
    }
    if *eflag == ARR_TYPE_NORM
        && let Some(attr) = attribute(zz, e, zz.E_arrowhead)
    {
        arrow_match_name(&attr, eflag);
    }
    if *sflag == ARR_TYPE_NORM
        && let Some(attr) = attribute(zz, e, zz.E_arrowtail)
    {
        arrow_match_name(&attr, sflag);
    }
    if zz.ed(e).conc_opp_flag {
        unimplemented!("1p2usipxeqlorwroqo37t3yfy: concentrated opposing edges");
    }
}

/// `arrow_length`: the length of the arrows in `flag`, scaled by `arrowsize`.
pub fn arrow_length(zz: &mut Globals, e: EdgeId, flag: i32) -> f64 {
    let mut lenfact = 0.0;
    for i in 0..NUMB_OF_ARROW_HEADS {
        let f = (flag >> (i * BITS_PER_ARROW)) & ((1 << BITS_PER_ARROW_TYPE) - 1);
        if let Some(arrowtype) = Arrowtypes.iter().find(|t| t.type_ == f) {
            lenfact += arrowtype.lenfact;
        }
    }
    10.0 * lenfact * late_double(zz, e, zz.E_arrowsz, 1.0, 0.0)
}

/// `inside` for arrows: whether `p` is within the arrow's length of its tip.
fn inside(_zz: &mut Globals, inside_context: &inside_t, p: pointf) -> bool {
    DIST2(p, inside_context.a_p) <= inside_context.a_r
}

/// `arrowEndClip`: shortens the spline's last Bézier piece by the arrowhead, which `spl.ep` records; returns the
/// index of the piece that now ends the spline.
pub fn arrowEndClip(
    zz: &mut Globals,
    e: EdgeId,
    ps: &mut [pointf],
    startp: i32,
    mut endp: i32,
    spl: &mut bezier,
    eflag: i32,
) -> i32 {
    let elen = arrow_length(zz, e, eflag);
    let elen2 = elen * elen;
    spl.eflag = eflag;
    spl.ep = ps[(endp + 3) as usize];
    if endp > startp && DIST2(ps[endp as usize], ps[(endp + 3) as usize]) < elen2 {
        endp -= 3;
    }
    let i = endp as usize;
    let mut sp = [spl.ep, ps[i + 2], ps[i + 1], ps[i]];
    let inside_context = inside_t {
        a_p: sp[0],
        a_r: elen2,
        ..inside_t::default()
    };
    bezier_clip(zz, &inside_context, inside, &mut sp, true);
    ps[i] = sp[3];
    ps[i + 1] = sp[2];
    ps[i + 2] = sp[1];
    ps[i + 3] = sp[0];
    endp
}

/// `arrowStartClip`: the same at the start, recorded in `spl.sp`; returns the index of the first piece.
pub fn arrowStartClip(
    zz: &mut Globals,
    e: EdgeId,
    ps: &mut [pointf],
    mut startp: i32,
    endp: i32,
    spl: &mut bezier,
    sflag: i32,
) -> i32 {
    let slen = arrow_length(zz, e, sflag);
    let slen2 = slen * slen;
    spl.sflag = sflag;
    spl.sp = ps[startp as usize];
    if endp > startp && DIST2(ps[startp as usize], ps[(startp + 3) as usize]) < slen2 {
        startp += 3;
    }
    let i = startp as usize;
    let mut sp = [ps[i + 3], ps[i + 2], ps[i + 1], spl.sp];
    let inside_context = inside_t {
        a_p: sp[3],
        a_r: slen2,
        ..inside_t::default()
    };
    bezier_clip(zz, &inside_context, inside, &mut sp, false);
    ps[i] = sp[3];
    ps[i + 1] = sp[2];
    ps[i + 2] = sp[1];
    ps[i + 3] = sp[0];
    startp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(name: &str) -> i32 {
        let mut flag = -1;
        arrow_match_name(name, &mut flag);
        flag
    }

    #[test]
    fn arrow_names_map_like_smetana() {
        assert_eq!(flags("normal"), ARR_TYPE_NORM);
        assert_eq!(flags("none"), ARR_TYPE_NONE);
        assert_eq!(flags("nonenormal"), ARR_TYPE_GAP | ARR_TYPE_NORM);
        assert_eq!(flags("vee"), ARR_TYPE_NONE);
    }
}
