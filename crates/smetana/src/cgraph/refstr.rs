//! `refstr.c`: reference-counted string interning, one dictionary per root graph.
//!
//! Interning decides object ids: a named object's id is the `CString` uid of its name's interned copy, made when
//! the string is first interned (or re-interned after its count dropped to zero). So every string Java interns is
//! interned here too, in the same order, attribute values and record names included.

use crate::cdt::{DtArg, JStr};
use crate::core::Globals;
use crate::core::ids::{ClosId, GraphId, StrId};
use crate::h::cgraph::refstr_t;

/// `HTML_BIT`: set in the count of strings interned as HTML, which Smetana never does.
const HTML_BIT: i32 = 1 << 31;

fn refdict(zz: &Globals, g: GraphId) -> ClosId {
    zz.graphs[g].clos
}

fn refsymbind(zz: &mut Globals, clos: ClosId, s: &str) -> Option<StrId> {
    zz.closes[clos]
        .strdict
        .dtsearch(DtArg::template(JStr(s.to_owned())))
}

/// `agstrbind`: the interned copy of `s`, if there is one.
pub fn agstrbind(zz: &mut Globals, g: GraphId, s: &str) -> Option<StrId> {
    let clos = refdict(zz, g);
    refsymbind(zz, clos, s)
}

/// `agstrdup`: interns `s`, or counts one more reference to its interned copy.
pub fn agstrdup(zz: &mut Globals, g: GraphId, s: &str) -> StrId {
    let clos = refdict(zz, g);
    if let Some(r) = refsymbind(zz, clos, s) {
        zz.refstrs[r].refcnt += 1;
        return r;
    }
    let uid = zz.next_cstring_uid();
    let r = zz.refstrs.push(refstr_t {
        s: s.to_owned(),
        refcnt: 1,
        uid,
    });
    zz.closes[clos].strdict.dtinsert(r, JStr(s.to_owned()));
    r
}

/// `agstrfree`: drops a reference; the last one removes the string, so interning it again makes a new copy.
pub fn agstrfree(zz: &mut Globals, g: GraphId, s: Option<StrId>) -> i32 {
    let Some(s) = s else { return -1 };
    let clos = refdict(zz, g);
    let text = zz.refstrs[s].s.clone();
    let Some(r) = refsymbind(zz, clos, &text) else {
        return -1;
    };
    assert_eq!(r, s, "agstrfree of a string that is not the interned copy");
    zz.refstrs[r].refcnt -= 1;
    if zz.refstrs[r].refcnt == 0 {
        zz.closes[clos].strdict.dtdelete(r, JStr(text));
    }
    0
}

/// `aghtmlstr`: whether `s` was interned as an HTML string.
pub fn aghtmlstr(zz: &Globals, s: StrId) -> i32 {
    zz.refstrs[s].refcnt & HTML_BIT
}
