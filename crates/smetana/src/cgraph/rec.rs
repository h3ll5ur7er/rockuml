//! `rec.c`: records attached to objects. Java finds them by name in a list; here each record is a field of the
//! object and binding only marks it present, but still interns the record's name as Java does.

use super::Agobj;
use super::obj::agraphof;
use super::refstr::agstrdup;
use crate::core::Globals;
use crate::h::cgraph::{Agattr_s, REC_ATTR, REC_DATADICT, REC_INFO};

/// The records cgraph and dot bind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rec {
    /// `AgDataRecName` (`_AG_strdata`): the attribute values.
    Attr,
    /// `DataDictName` (`_AG_datadict`): a graph's attribute declarations.
    DataDict,
    /// `Agraphinfo_t`, `Agnodeinfo_t` or `Agedgeinfo_t`: dot's record.
    Info,
}

impl Rec {
    fn bit(self) -> u8 {
        match self {
            Rec::Attr => REC_ATTR,
            Rec::DataDict => REC_DATADICT,
            Rec::Info => REC_INFO,
        }
    }

    fn name(self, obj: Agobj) -> &'static str {
        match (self, obj) {
            (Rec::Attr, _) => "_AG_strdata",
            (Rec::DataDict, _) => "_AG_datadict",
            (Rec::Info, Agobj::Graph(_)) => "Agraphinfo_t",
            (Rec::Info, Agobj::Node(_)) => "Agnodeinfo_t",
            (Rec::Info, Agobj::Edge(_)) => "Agedgeinfo_t",
        }
    }
}

fn recs_mut(zz: &mut Globals, obj: Agobj) -> &mut u8 {
    match obj {
        Agobj::Graph(g) => &mut zz.graphs[g].recs,
        Agobj::Node(n) => &mut zz.nodes[n].recs,
        Agobj::Edge(e) => &mut zz.edgepair_mut(e).recs,
    }
}

/// `aggetrec(obj, name) != NULL`.
pub(crate) fn aggetrec(zz: &mut Globals, obj: impl Into<Agobj>, rec: Rec) -> bool {
    let obj = obj.into();
    *recs_mut(zz, obj) & rec.bit() != 0
}

/// `agbindrec`: binds a fresh record unless one is bound already.
pub fn agbindrec(zz: &mut Globals, obj: impl Into<Agobj>, rec: Rec) {
    let obj = obj.into();
    if aggetrec(zz, obj, rec) {
        return;
    }
    let g = agraphof(zz, obj);
    agstrdup(zz, g, rec.name(obj));
    *recs_mut(zz, obj) |= rec.bit();
    match (rec, obj) {
        (Rec::Attr, Agobj::Graph(g)) => zz.graphs[g].attr = Some(Agattr_s::default()),
        (Rec::Attr, Agobj::Node(n)) => zz.nodes[n].attr = Some(Agattr_s::default()),
        (Rec::Attr, Agobj::Edge(e)) => zz.edgepair_mut(e).attr = Some(Agattr_s::default()),
        (Rec::DataDict, Agobj::Graph(_)) | (Rec::Info, _) => {}
        (Rec::DataDict, _) => unreachable!("only graphs have attribute declarations"),
    }
}
