//! `attr.c`: string attributes. A root graph declares each attribute (`Agsym_s`) in one of three dictionaries
//! (graphs, nodes, edges); a subgraph's dictionaries view its parent's and hold its local defaults. Every object
//! stores its values in an array indexed by the declaration's id.

use super::apply::agapply;
use super::edge::{agfstout, agnxtout};
use super::graph::{agopen, agparent};
use super::node::{agfstnode, agnxtnode};
use super::obj::{agraphof, agroot};
use super::rec::{Rec, agbindrec, aggetrec};
use super::refstr::{agstrdup, agstrfree};
use super::{AGINEDGE, AGNODE, AGOUTEDGE, AGRAPH, Agobj};
use crate::cdt::{DT_INSERT, DT_OSET, DtArg, JStr};
use crate::core::Globals;
use crate::core::ids::{DictId, GraphId, NodeId, StrId, SymId};
use crate::h::cgraph::{Agattr_s, Agdatadict_s, Agsym_s, ProtoDesc};

/// Value arrays start with room for this many attributes.
const MINATTR: usize = 4;

/// `agdatadict`: `g`'s attribute declarations. Graphs always get them when opened.
fn agdatadict(zz: &Globals, g: GraphId) -> Option<Agdatadict_s> {
    zz.graphs[g].datadict
}

/// `agdictof`: the dictionary of `g` declaring attributes of `kind` objects.
fn agdictof(zz: &Globals, g: GraphId, kind: i32) -> Option<DictId> {
    let dd = agdatadict(zz, g)?;
    Some(match kind {
        AGRAPH => dd.dict_g,
        AGNODE => dd.dict_n,
        AGINEDGE | AGOUTEDGE => dd.dict_e,
        _ => panic!("agdictof: unknown kind {kind}"),
    })
}

/// `agnewsym`.
fn agnewsym(zz: &mut Globals, g: GraphId, name: &str, value: &str, id: i32) -> SymId {
    let name = agstrdup(zz, g, name);
    let defval = agstrdup(zz, g, value);
    zz.syms.push(Agsym_s { name, defval, id })
}

fn sym_key(zz: &Globals, sym: SymId) -> JStr {
    JStr(zz.agstr(zz.syms[sym].name).to_owned())
}

/// `agmakedatadict`: opens `g`'s three dictionaries, viewing the parent's.
fn agmakedatadict(zz: &mut Globals, g: GraphId) -> Agdatadict_s {
    agbindrec(zz, g, Rec::DataDict);
    let dd = Agdatadict_s {
        dict_n: zz.attr_dicts.dtopen(DT_OSET),
        dict_e: zz.attr_dicts.dtopen(DT_OSET),
        dict_g: zz.attr_dicts.dtopen(DT_OSET),
    };
    zz.graphs[g].datadict = Some(dd);
    if let Some(par) = agparent(zz, g) {
        let parent_dd = agdatadict(zz, par).expect("parent declarations");
        zz.attr_dicts.dtview(dd.dict_n, Some(parent_dd.dict_n));
        zz.attr_dicts.dtview(dd.dict_e, Some(parent_dd.dict_e));
        zz.attr_dicts.dtview(dd.dict_g, Some(parent_dd.dict_g));
    } else if zz.ProtoGraph.is_some_and(|p| p != g) {
        unimplemented!("agcopydict: a root graph opened after the prototype graph");
    }
    dd
}

/// `agdictsym`: the declaration of `name`, searching the view path.
fn agdictsym(zz: &mut Globals, dict: DictId, name: &str) -> Option<SymId> {
    zz.attr_dicts
        .dtsearch(dict, DtArg::template(JStr(name.to_owned())))
}

/// `aglocaldictsym`: the declaration of `name` in `dict` itself.
fn aglocaldictsym(zz: &mut Globals, dict: DictId, name: &str) -> Option<SymId> {
    let view = zz.attr_dicts.dtview(dict, None);
    let rv = agdictsym(zz, dict, name);
    zz.attr_dicts.dtview(dict, view);
    rv
}

/// `agattrsym`: the declaration of `name` for `obj`'s kind.
fn agattrsym(zz: &mut Globals, obj: Agobj, name: &str) -> Option<SymId> {
    let dict = agattrrec(zz, obj)?.dict?;
    agdictsym(zz, dict, name)
}

/// `topdictsize`: how many attributes the root graph declares for `obj`'s kind.
fn topdictsize(zz: &mut Globals, obj: Agobj) -> i32 {
    let root = agroot(zz, agraphof(zz, obj));
    match agdictof(zz, root, zz.tag(obj).objtype) {
        Some(d) => zz.attr_dicts.dt(d).dtsize_(),
        None => 0,
    }
}

fn attr_mut(zz: &mut Globals, obj: Agobj) -> &mut Agattr_s {
    let attr = match obj {
        Agobj::Graph(g) => &mut zz.graphs[g].attr,
        Agobj::Node(n) => &mut zz.nodes[n].attr,
        Agobj::Edge(e) => &mut zz.edgepair_mut(e).attr,
    };
    attr.as_mut().expect("object without attribute record")
}

/// `agmakeattrs`: gives `obj` the default values that `context` declares.
fn agmakeattrs(zz: &mut Globals, context: GraphId, obj: Agobj) {
    agbindrec(zz, obj, Rec::Attr);
    let objtype = zz.tag(obj).objtype;
    let datadict = agdictof(zz, context, objtype).expect("context declarations");
    if agattrrec(zz, obj).is_some_and(|a| a.dict.is_some()) {
        return;
    }
    let root = agroot(zz, context);
    let dict = agdictof(zz, root, objtype);
    let sz = usize::try_from(topdictsize(zz, obj))
        .expect("dictionary size")
        .max(MINATTR);
    let rec = attr_mut(zz, obj);
    rec.dict = dict;
    rec.str = vec![None; sz];
    // Doesn't call agxset(), so no obj-modified callbacks occur.
    let g = agraphof(zz, obj);
    let mut sym = zz.attr_dicts.dtfirst(datadict);
    while let Some(s) = sym {
        let Agsym_s { id, defval, .. } = zz.syms[s];
        let text = zz.agstr(defval).to_owned();
        let value = agstrdup(zz, g, &text);
        attr_mut(zz, obj).str[id as usize] = Some(value);
        let key = sym_key(zz, s);
        sym = zz.attr_dicts.dtnext(datadict, s, key);
    }
}

/// `agattrrec`: `obj`'s values, if bound.
fn agattrrec(zz: &mut Globals, obj: Agobj) -> Option<&Agattr_s> {
    if !aggetrec(zz, obj, Rec::Attr) {
        return None;
    }
    match obj {
        Agobj::Graph(g) => zz.graphs[g].attr.as_ref(),
        Agobj::Node(n) => zz.nodes[n].attr.as_ref(),
        Agobj::Edge(e) => zz.edgepair(e).attr.as_ref(),
    }
}

/// `addattr`: gives `obj` the default of a newly declared attribute.
fn addattr(zz: &mut Globals, g: GraphId, obj: Agobj, sym: SymId) {
    let Agsym_s { id, defval, .. } = zz.syms[sym];
    let text = zz.agstr(defval).to_owned();
    let value = agstrdup(zz, g, &text);
    let attr = attr_mut(zz, obj);
    if id as usize >= MINATTR {
        attr.str.push(None);
    }
    attr.str[id as usize] = Some(value);
}

/// `setattr`: declares `name` for `kind` objects of `g` with default `value`, or changes its default.
fn setattr(zz: &mut Globals, g: GraphId, kind: i32, name: &str, value: &str) -> SymId {
    let root = agroot(zz, g);
    let ldict = agdictof(zz, g, kind).expect("graph declarations");
    let rv = if let Some(lsym) = aglocaldictsym(zz, ldict, name) {
        // Update the old local definition.
        let old = zz.syms[lsym].defval;
        agstrfree(zz, g, Some(old));
        zz.syms[lsym].defval = agstrdup(zz, g, value);
        lsym
    } else if let Some(psym) = agdictsym(zz, ldict, name) {
        // A new local definition.
        let id = zz.syms[psym].id;
        let lsym = agnewsym(zz, g, name, value, id);
        zz.attr_dicts.dtinsert(ldict, lsym, JStr(name.to_owned()));
        lsym
    } else {
        // A new global definition.
        let rdict = agdictof(zz, root, kind).expect("root declarations");
        let id = zz.attr_dicts.dt(rdict).dtsize_();
        let rsym = agnewsym(zz, g, name, value, id);
        zz.attr_dicts.dtinsert(rdict, rsym, JStr(name.to_owned()));
        match kind {
            AGRAPH => {
                agapply(zz, root, &mut |zz, g| addattr(zz, g, g.into(), rsym), true);
            }
            AGNODE => {
                let mut n = agfstnode(zz, root);
                while let Some(nn) = n {
                    addattr(zz, g, nn.into(), rsym);
                    n = agnxtnode(zz, root, nn);
                }
            }
            _ => {
                let mut n = agfstnode(zz, root);
                while let Some(nn) = n {
                    let mut e = agfstout(zz, root, nn);
                    while let Some(ee) = e {
                        addattr(zz, g, ee.into(), rsym);
                        e = agnxtout(zz, root, ee);
                    }
                    n = agnxtnode(zz, root, nn);
                }
            }
        }
        rsym
    };
    if kind == AGRAPH {
        agxset(zz, g, rv, value);
    }
    rv
}

/// `getattr`: the declaration of `name` for `kind` objects of `g`.
fn getattr(zz: &mut Globals, g: GraphId, kind: i32, name: &str) -> Option<SymId> {
    let dict = agdictof(zz, g, kind)?;
    agdictsym(zz, dict, name)
}

/// `agattr`: with a value, declares `name` (or sets its default); without, looks its declaration up. Without a
/// graph it works on the prototype graph, which it opens on first use.
pub fn agattr(
    zz: &mut Globals,
    g: Option<GraphId>,
    kind: i32,
    name: &str,
    value: Option<&str>,
) -> Option<SymId> {
    let g = match (g, zz.ProtoGraph) {
        (Some(g), _) | (None, Some(g)) => g,
        (None, None) => {
            let proto = agopen(zz, None, ProtoDesc);
            zz.ProtoGraph = Some(proto);
            proto
        }
    };
    match value {
        Some(value) => Some(setattr(zz, g, kind, name, value)),
        None => getattr(zz, g, kind, name),
    }
}

/// `agraphattr_init`.
pub(crate) fn agraphattr_init(zz: &mut Globals, g: GraphId) {
    zz.graphs[g].desc.has_attrs = 1;
    agmakedatadict(zz, g);
    let context = agparent(zz, g).unwrap_or(g);
    agmakeattrs(zz, context, g.into());
}

/// `agnodeattr_init`.
pub(crate) fn agnodeattr_init(zz: &mut Globals, g: GraphId, n: NodeId) {
    if agattrrec(zz, n.into()).is_none_or(|a| a.dict.is_none()) {
        agmakeattrs(zz, g, n.into());
    }
}

/// `agedgeattr_init`.
pub(crate) fn agedgeattr_init(zz: &mut Globals, g: GraphId, e: crate::core::ids::EdgeId) {
    if agattrrec(zz, e.into()).is_none_or(|a| a.dict.is_none()) {
        agmakeattrs(zz, g, e.into());
    }
}

/// `agget`: `obj`'s value of attribute `name`, or `None` if it is not declared.
pub fn agget(zz: &mut Globals, obj: impl Into<Agobj>, name: &str) -> Option<StrId> {
    let obj = obj.into();
    let sym = agattrsym(zz, obj, name)?;
    agxget(zz, obj, sym)
}

/// `agxget`: `obj`'s value of a declared attribute.
pub fn agxget(zz: &mut Globals, obj: impl Into<Agobj>, sym: SymId) -> Option<StrId> {
    let id = zz.syms[sym].id as usize;
    agattrrec(zz, obj.into())
        .expect("object without attribute record")
        .str[id]
}

/// `agxset`: sets `obj`'s value of a declared attribute; for a graph, also its local default.
pub fn agxset(zz: &mut Globals, obj: impl Into<Agobj>, sym: SymId, value: &str) -> i32 {
    let obj = obj.into();
    let g = agraphof(zz, obj);
    let Agsym_s { id, name, .. } = zz.syms[sym];
    let old = attr_mut(zz, obj).str[id as usize];
    agstrfree(zz, g, old);
    let v = agstrdup(zz, g, value);
    attr_mut(zz, obj).str[id as usize] = Some(v);
    let objtype = zz.tag(obj).objtype;
    if objtype == AGRAPH {
        // Also update the dictionary default.
        let dict = agdatadict(zz, g).expect("graph declarations").dict_g;
        let name = zz.agstr(name).to_owned();
        if let Some(lsym) = aglocaldictsym(zz, dict, &name) {
            let old = zz.syms[lsym].defval;
            agstrfree(zz, g, Some(old));
            zz.syms[lsym].defval = agstrdup(zz, g, value);
        } else {
            let lsym = agnewsym(zz, g, &name, value, id);
            zz.attr_dicts
                .searchf(dict, Some(DtArg::object(lsym, JStr(name))), DT_INSERT);
        }
    }
    0
}

/// `agsafeset`: sets `obj`'s attribute `name`, declaring it with default `def` first if needed.
pub fn agsafeset(
    zz: &mut Globals,
    obj: impl Into<Agobj>,
    name: &str,
    value: &str,
    def: &str,
) -> i32 {
    let obj = obj.into();
    let g = agraphof(zz, obj);
    let kind = zz.tag(obj).objtype;
    let a = agattr(zz, Some(g), kind, name, None)
        .or_else(|| agattr(zz, Some(g), kind, name, Some(def)))
        .expect("declared attribute");
    agxset(zz, obj, a, value)
}

/// `agfindgraphattr`.
pub(crate) fn agfindgraphattr(zz: &mut Globals, g: GraphId, name: &str) -> Option<SymId> {
    agattr(zz, Some(g), AGRAPH, name, None)
}

/// `agfindnodeattr`.
pub(crate) fn agfindnodeattr(zz: &mut Globals, g: GraphId, name: &str) -> Option<SymId> {
    agattr(zz, Some(g), AGNODE, name, None)
}

/// `agfindedgeattr`.
pub(crate) fn agfindedgeattr(zz: &mut Globals, g: GraphId, name: &str) -> Option<SymId> {
    agattr(zz, Some(g), super::AGEDGE, name, None)
}
