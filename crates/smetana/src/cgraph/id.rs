//! `id.c`: object ids. A named object's id is the uid of its interned name (even); an anonymous one gets the next
//! odd number from `ctr`. Subgraphs are kept in id order, so ids decide the order of `agfstsubg`/`agnxtsubg`.

use super::refstr::{agstrbind, agstrdup};
use super::{AGEDGE, Agobj};
use crate::core::Globals;
use crate::core::ids::GraphId;

/// `idmap`: the id for `str`, interning it when `createflag` is set; without a string, a new anonymous id.
fn idmap(zz: &mut Globals, g: GraphId, str: Option<&str>, createflag: bool) -> i32 {
    let Some(str) = str else {
        let id = zz.ctr;
        zz.ctr += 2;
        return id;
    };
    let s = if createflag {
        Some(agstrdup(zz, g, str))
    } else {
        agstrbind(zz, g, str)
    };
    // Memory.identityHashCode: 0 for NULL, else the uid, remembered so that idprint can map it back.
    s.map_or(0, |s| {
        let uid = zz.refstrs[s].uid;
        zz.all.insert(uid, s);
        uid
    })
}

/// `agmapnametoid`: the id of the object called `str`, or `None` where C returns 0 (no name and no creation).
/// Like Smetana, it does not support cgraph's internal `%<id>` names.
pub fn agmapnametoid(
    zz: &mut Globals,
    g: GraphId,
    str: Option<&str>,
    createflag: bool,
) -> Option<i32> {
    match str {
        Some(s) if s.starts_with('%') => unimplemented!("aginternalmaplookup"),
        Some(_) => Some(idmap(zz, g, str, createflag)),
        None if createflag => Some(idmap(zz, g, None, true)),
        None => None,
    }
}

/// `agnameof`: a node's or graph's name, an edge's key, or `None` for an anonymous edge.
pub fn agnameof(zz: &Globals, obj: impl Into<Agobj>) -> Option<String> {
    let obj = obj.into();
    let tag = *zz.tag(obj);
    // idprint: even ids are string uids.
    if tag.id % 2 == 0 {
        let s = zz
            .all
            .get(&tag.id)
            .unwrap_or_else(|| panic!("no string for object id {}", tag.id));
        return Some(zz.refstrs[*s].s.clone());
    }
    if tag.objtype == AGEDGE {
        None
    } else {
        Some(format!("%{}", tag.id))
    }
}
