//! `dtview.c`: view paths, which make a dictionary also show the objects of the dictionaries below it. cgraph
//! uses them for attribute declarations: a subgraph's dictionaries view its parent's.

use super::{
    DT_CLEAR, DT_DELETE, DT_FIRST, DT_INSERT, DT_LAST, DT_MATCH, DT_NEXT, DT_OBAG, DT_OSET,
    DT_PREV, DT_RENEW, DT_SEARCH, Dt, DtArg, DtKey, Searchf,
};
use crate::core::ids::{Arena, DictId};

/// Dictionaries that can view each other, so they live in one arena.
pub(crate) struct Dicts<O, K>(Arena<DictId, Dt<O, K>>);

impl<O, K> Default for Dicts<O, K> {
    fn default() -> Self {
        Self(Arena::default())
    }
}

impl<O: Copy + PartialEq, K: DtKey> Dicts<O, K> {
    pub(crate) fn dtopen(&mut self, meth: i32) -> DictId {
        self.0.push(Dt::dtopen(meth))
    }

    pub(crate) fn dt(&mut self, dt: DictId) -> &mut Dt<O, K> {
        &mut self.0[dt]
    }

    /// `(*dt->searchf)(dt, obj, type)`.
    pub(crate) fn searchf(
        &mut self,
        dt: DictId,
        arg: Option<DtArg<O, K>>,
        type_: i32,
    ) -> Option<(O, K)> {
        match self.0[dt].searchf {
            Searchf::Dttree => self.0[dt].dttree(arg, type_),
            Searchf::Dtvsearch => self.dtvsearch(dt, arg, type_),
        }
    }

    pub(crate) fn dtinsert(&mut self, dt: DictId, obj: O, key: K) -> Option<O> {
        self.searchf(dt, Some(DtArg::object(obj, key)), DT_INSERT)
            .map(|(o, _)| o)
    }

    pub(crate) fn dtsearch(&mut self, dt: DictId, arg: DtArg<O, K>) -> Option<O> {
        self.searchf(dt, Some(arg), DT_SEARCH).map(|(o, _)| o)
    }

    pub(crate) fn dtfirst(&mut self, dt: DictId) -> Option<O> {
        self.searchf(dt, None, DT_FIRST).map(|(o, _)| o)
    }

    pub(crate) fn dtnext(&mut self, dt: DictId, obj: O, key: K) -> Option<O> {
        self.searchf(dt, Some(DtArg::object(obj, key)), DT_NEXT)
            .map(|(o, _)| o)
    }

    /// `dtvsearch`: searches along the view path.
    fn dtvsearch(&mut self, dt: DictId, arg: Option<DtArg<O, K>>, type_: i32) -> Option<(O, K)> {
        // These operations only happen at the top level.
        if type_ & (DT_INSERT | DT_DELETE | DT_CLEAR | DT_RENEW) != 0 {
            return self.0[dt].dttree(arg, type_);
        }
        let ordered = self.0[dt].meth & (DT_OBAG | DT_OSET) != 0;
        if type_ & (DT_MATCH | DT_SEARCH) != 0 || (type_ & (DT_FIRST | DT_LAST) != 0 && !ordered) {
            let mut d = Some(dt);
            let mut o = None;
            while let Some(di) = d {
                o = self.0[di].dttree(arg.clone(), type_);
                if o.is_some() {
                    break;
                }
                d = self.0[di].view;
            }
            self.0[dt].walk = d;
            return o;
        }
        assert!(ordered, "dtvsearch: unordered methods");
        if type_ & (DT_FIRST | DT_LAST | DT_NEXT | DT_PREV) == 0 {
            return None;
        }
        // The answer is the best of each dictionary's answer: the smallest for FIRST/NEXT, the largest for
        // LAST/PREV, the dictionary nearest the top on ties.
        let mut n: Option<(O, K)> = None;
        let mut p = None;
        let mut d = Some(dt);
        while let Some(di) = d {
            d = self.0[di].view;
            let Some(o) = self.0[di].dttree(arg.clone(), type_) else {
                continue;
            };
            let better = match &n {
                None => true,
                Some((_, nk)) => {
                    let cmp = o.1.dtcmp(nk);
                    (type_ & (DT_NEXT | DT_FIRST) != 0 && cmp < 0)
                        || (type_ & (DT_PREV | DT_LAST) != 0 && cmp > 0)
                }
            };
            if better {
                p = Some(di);
                n = Some(o);
            }
        }
        self.0[dt].walk = p;
        n
    }

    /// `dtview(dt, view)`: makes `dt` view `view` (or nothing), returning the new view, or for `None` the old one.
    pub(crate) fn dtview(&mut self, dt: DictId, view: Option<DictId>) -> Option<DictId> {
        if let Some(v) = view {
            assert_eq!(
                self.0[v].meth, self.0[dt].meth,
                "dtview: views must use the same method"
            );
        }
        // Make sure there won't be a cycle.
        let mut d = view;
        while let Some(di) = d {
            if di == dt {
                return None;
            }
            d = self.0[di].view;
        }
        // No more viewing the lower dictionary.
        let old = self.0[dt].view;
        if let Some(o) = old {
            self.0[o].nview -= 1;
        }
        self.0[dt].view = None;
        self.0[dt].walk = None;
        let Some(v) = view else {
            self.0[dt].searchf = Searchf::Dttree;
            return old;
        };
        self.0[dt].view = Some(v);
        self.0[dt].searchf = Searchf::Dtvsearch;
        self.0[v].nview += 1;
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DT_OSET, DtArg};
    use super::Dicts;

    #[test]
    fn a_view_shows_the_union_with_the_top_winning_ties() {
        let mut dicts: Dicts<u32, i32> = Dicts::default();
        let parent = dicts.dtopen(DT_OSET);
        let child = dicts.dtopen(DT_OSET);
        for (obj, key) in [(0, 10), (1, 20), (2, 30)] {
            dicts.dtinsert(parent, obj, key);
        }
        for (obj, key) in [(3, 20), (4, 25)] {
            dicts.dtinsert(child, obj, key);
        }
        dicts.dtview(child, Some(parent));
        let keys = [10, 20, 30, 20, 25];
        let mut seen = Vec::new();
        let mut o = dicts.dtfirst(child);
        while let Some(obj) = o {
            seen.push(obj);
            o = dicts.dtnext(child, obj, keys[obj as usize]);
        }
        assert_eq!(seen, vec![0, 3, 4, 2]);
        assert_eq!(dicts.dtsearch(child, DtArg::template(30)), Some(2));
        assert_eq!(dicts.dtsearch(child, DtArg::template(20)), Some(3));

        // Without the view only the child's own objects are visible.
        let view = dicts.dtview(child, None);
        assert_eq!(view, Some(parent));
        assert_eq!(dicts.dtsearch(child, DtArg::template(30)), None);
        dicts.dtview(child, view);
        assert_eq!(dicts.dtsearch(child, DtArg::template(10)), Some(0));
    }

    #[test]
    fn views_cannot_form_cycles() {
        let mut dicts: Dicts<u32, i32> = Dicts::default();
        let a = dicts.dtopen(DT_OSET);
        let b = dicts.dtopen(DT_OSET);
        dicts.dtview(a, Some(b));
        assert_eq!(dicts.dtview(b, Some(a)), None);
    }
}
