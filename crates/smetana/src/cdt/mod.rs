//! cdt, Kiem-Phong Vo's container library, reduced to what cgraph and xlabels use: ordered sets (`Dttree`,
//! `DT_OSET`) and ordered bags (`Dtobag`, `DT_OBAG`) kept in a splay tree, with view paths.
//!
//! The splay tree is ported operation for operation, because its shape decides which of several equal keys a
//! search finds (cgraph's wildcard edge lookups) and in which order equal keys of a bag come out.
//!
//! A dictionary owns the links of its tree. An object is any `Copy` handle and each link also caches the object's
//! key, which the discipline would read from the object in C (`_DTKEY`). Keys never change while an object is in a
//! dictionary, so the cache is exact; it lets the tree compare keys without reaching back into the graph. Calls
//! that pass an object therefore pass its key too ([`DtArg`]); a search template is a key without an object.
//!
//! A set can be detached from the dictionary as a bare tree ([`Dt::dtextract`]) and put back later
//! ([`Dt::dtrestore`]); cgraph keeps every node's edge sets that way, all in one dictionary per graph.

mod dttree;
mod dtview;

pub(crate) use dtview::Dicts;

/// The discipline's comparison function (`comparf`, or `strcmp` on string keys). Only the sign matters.
pub(crate) trait DtKey: Clone {
    fn dtcmp(&self, other: &Self) -> i32;
}

/// Integer keys (ids and sequence numbers) compare by `k1 - k2` in wrapping `int` arithmetic, as cgraph's
/// comparators and xlabels' `icompare` do.
impl DtKey for i32 {
    fn dtcmp(&self, other: &Self) -> i32 {
        self.wrapping_sub(*other).signum()
    }
}

/// A string key, ordered by Java's `strcmp` (`AgDataDictDisc` and `Refstrdisc` have no `comparf`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JStr(pub(crate) String);

impl DtKey for JStr {
    fn dtcmp(&self, other: &Self) -> i32 {
        crate::core::jutils::strcmp(&self.0, &other.0)
    }
}

// Dictionary methods (`Dtmethod_t.type`).
pub(crate) const DT_OSET: i32 = 0o4;
pub(crate) const DT_OBAG: i32 = 0o10;

// Search types.
pub(crate) const DT_INSERT: i32 = 0o1;
pub(crate) const DT_DELETE: i32 = 0o2;
pub(crate) const DT_SEARCH: i32 = 0o4;
pub(crate) const DT_NEXT: i32 = 0o10;
pub(crate) const DT_PREV: i32 = 0o20;
pub(crate) const DT_RENEW: i32 = 0o40;
pub(crate) const DT_CLEAR: i32 = 0o100;
pub(crate) const DT_FIRST: i32 = 0o200;
pub(crate) const DT_LAST: i32 = 0o400;
pub(crate) const DT_MATCH: i32 = 0o1000;
pub(crate) const DT_ATTACH: i32 = 0o4000;
pub(crate) const DT_DETACH: i32 = 0o10000;

/// Set in `Dt.type_` when the last search found its object.
pub(crate) const DT_FOUND: i32 = 0o100000;

/// A `Dtlink_t*` into one dictionary's links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinkId(u32);

impl LinkId {
    /// `dttree`'s local `link` header, reused by every call.
    const HEADER: LinkId = LinkId(0);

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// A `Dtlink_t`. As in cdt, `left` is `hl._left` and `right` is `right`.
#[derive(Clone, Copy, Debug, Default)]
struct Dtlink {
    left: Option<LinkId>,
    right: Option<LinkId>,
}

/// The object an operation is about (`void* obj`): an object with its key, or a bare key to search for.
#[derive(Clone, Debug)]
pub(crate) struct DtArg<O, K> {
    pub(crate) obj: Option<O>,
    pub(crate) key: K,
}

impl<O, K> DtArg<O, K> {
    pub(crate) fn object(obj: O, key: K) -> Self {
        Self {
            obj: Some(obj),
            key,
        }
    }

    pub(crate) fn template(key: K) -> Self {
        Self { obj: None, key }
    }
}

/// `Dtdata_t`: `type` (the method plus `DT_FLATTEN`, which nothing sets here), the finger `here` (the tree's root),
/// and `size`, which is -1 when unknown.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dtdata {
    pub(crate) type_: i32,
    pub(crate) here: Option<LinkId>,
    pub(crate) size: i32,
}

/// How `searchf` is implemented: the method's own search, or the view-path search once a view is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Searchf {
    Dttree,
    Dtvsearch,
}

/// A dictionary (`Dt_t`).
#[derive(Debug)]
pub(crate) struct Dt<O, K> {
    links: Vec<Dtlink>,
    /// The object and key of each link; the header's entry is `None`.
    objs: Vec<Option<(O, K)>>,
    pub(crate) data: Dtdata,
    /// `meth->type`.
    pub(crate) meth: i32,
    pub(crate) type_: i32,
    pub(crate) searchf: Searchf,
    pub(crate) view: Option<crate::core::ids::DictId>,
}

impl<O: Copy + PartialEq, K: DtKey> Dt<O, K> {
    /// `dtopen(disc, meth)`, with `meth` either [`DT_OSET`] (`Dttree`) or [`DT_OBAG`] (`Dtobag`).
    pub(crate) fn dtopen(meth: i32) -> Self {
        Self {
            links: vec![Dtlink::default()],
            objs: vec![None],
            data: Dtdata {
                type_: meth,
                here: None,
                size: 0,
            },
            meth,
            type_: 0,
            searchf: Searchf::Dttree,
            view: None,
        }
    }

    pub(crate) fn dtinsert(&mut self, obj: O, key: K) -> Option<O> {
        self.dttree(Some(DtArg::object(obj, key)), DT_INSERT)
            .map(|(o, _)| o)
    }

    pub(crate) fn dtsearch(&mut self, arg: DtArg<O, K>) -> Option<O> {
        self.dttree(Some(arg), DT_SEARCH).map(|(o, _)| o)
    }

    pub(crate) fn dtfirst(&mut self) -> Option<O> {
        self.dttree(None, DT_FIRST).map(|(o, _)| o)
    }

    pub(crate) fn dtnext(&mut self, obj: O, key: K) -> Option<O> {
        self.dttree(Some(DtArg::object(obj, key)), DT_NEXT)
            .map(|(o, _)| o)
    }

    pub(crate) fn dtdelete(&mut self, obj: O, key: K) -> Option<O> {
        self.dttree(Some(DtArg::object(obj, key)), DT_DELETE)
            .map(|(o, _)| o)
    }

    /// `dtsize`: the number of objects, counted when unknown.
    pub(crate) fn dtsize_(&mut self) -> i32 {
        if self.data.size < 0 && (self.data.type_ & (DT_OSET | DT_OBAG)) != 0 {
            self.data.size = self.treecount(self.data.here);
        }
        self.data.size
    }

    /// `treecount`, without recursion: splay trees can be as deep as they are big.
    fn treecount(&self, e: Option<LinkId>) -> i32 {
        let mut count = 0;
        let mut stack: Vec<LinkId> = e.into_iter().collect();
        while let Some(l) = stack.pop() {
            count += 1;
            let link = self.links[l.index()];
            stack.extend(link.left);
            stack.extend(link.right);
        }
        count
    }

    /// `dtextract`: detaches the tree and returns it; the dictionary is left empty.
    pub(crate) fn dtextract(&mut self) -> Option<LinkId> {
        let list = self.data.here;
        self.data.size = 0;
        self.data.here = None;
        list
    }

    /// `dtrestore`: installs a tree that [`Dt::dtextract`] returned. Like cdt it refuses (returns -1) unless the
    /// dictionary is empty, and does nothing for an empty tree.
    pub(crate) fn dtrestore(&mut self, list: Option<LinkId>) -> i32 {
        let Some(list) = list else {
            // Restoring a flattened dictionary; nothing ever flattens one.
            return -1;
        };
        if self.data.size != 0 {
            return -1;
        }
        self.data.here = Some(list);
        self.data.size = -1;
        0
    }

    fn new_link(&mut self, obj: O, key: K) -> LinkId {
        self.links.push(Dtlink::default());
        self.objs.push(Some((obj, key)));
        LinkId(u32::try_from(self.links.len() - 1).expect("dictionary overflow"))
    }

    fn obj(&self, l: LinkId) -> &(O, K) {
        self.objs[l.index()]
            .as_ref()
            .expect("the header link holds no object")
    }
}
