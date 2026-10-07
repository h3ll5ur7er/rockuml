//! `dttree.c`: the top-down splay tree behind `Dttree` and `Dtobag`.

use std::cmp::Ordering;

use super::{
    DT_ATTACH, DT_CLEAR, DT_DELETE, DT_DETACH, DT_FIRST, DT_FOUND, DT_INSERT, DT_LAST, DT_MATCH,
    DT_NEXT, DT_OBAG, DT_OSET, DT_PREV, DT_RENEW, DT_SEARCH, Dt, DtArg, DtKey, LinkId,
};

/// Where `dttree`'s gotos lead: `has_root` returns the root, `no_root` rebuilds the tree without one.
enum Outcome {
    HasRoot(LinkId),
    NoRoot,
}

impl<O: Copy + PartialEq, K: DtKey> Dt<O, K> {
    fn left(&self, l: LinkId) -> Option<LinkId> {
        self.links[l.index()].left
    }

    fn right(&self, l: LinkId) -> Option<LinkId> {
        self.links[l.index()].right
    }

    fn set_left(&mut self, l: LinkId, v: Option<LinkId>) {
        self.links[l.index()].left = v;
    }

    fn set_right(&mut self, l: LinkId, v: Option<LinkId>) {
        self.links[l.index()].right = v;
    }

    /// `rrotate(x, y)`.
    fn rrotate(&mut self, x: LinkId, y: LinkId) {
        self.set_left(x, self.right(y));
        self.set_right(y, Some(x));
    }

    /// `lrotate(x, y)`.
    fn lrotate(&mut self, x: LinkId, y: LinkId) {
        self.set_right(x, self.left(y));
        self.set_left(y, Some(x));
    }

    /// `rlink(r, x)`: hangs `x` on the RIGHT tree.
    fn rlink(&mut self, r: LinkId, x: LinkId) -> LinkId {
        self.set_left(r, Some(x));
        x
    }

    /// `llink(l, x)`: hangs `x` on the LEFT tree.
    fn llink(&mut self, l: LinkId, x: LinkId) -> LinkId {
        self.set_right(l, Some(x));
        x
    }

    fn cmp(&self, key: &K, l: LinkId) -> i32 {
        key.dtcmp(&self.obj(l).1)
    }

    /// `dttree`'s `do_search` loop, a top-down splay: the links smaller than `key` go on the LEFT tree through
    /// `l`, the larger ones on the RIGHT tree through `r`. Returns the link with an equal key, if any.
    fn splay_search(
        &mut self,
        key: &K,
        mut rt: LinkId,
        l: &mut LinkId,
        r: &mut LinkId,
    ) -> Option<LinkId> {
        loop {
            let cmp = self.cmp(key, rt);
            if cmp == 0 {
                return Some(rt);
            }
            if cmp < 0 {
                let Some(t) = self.left(rt) else {
                    *r = self.rlink(*r, rt);
                    return None;
                };
                match self.cmp(key, t).cmp(&0) {
                    Ordering::Less => {
                        self.rrotate(rt, t);
                        *r = self.rlink(*r, t);
                        rt = self.left(t)?;
                    }
                    Ordering::Equal => {
                        *r = self.rlink(*r, rt);
                        return Some(t);
                    }
                    Ordering::Greater => {
                        *l = self.llink(*l, t);
                        *r = self.rlink(*r, rt);
                        rt = self.right(t)?;
                    }
                }
            } else {
                let Some(t) = self.right(rt) else {
                    *l = self.llink(*l, rt);
                    return None;
                };
                match self.cmp(key, t).cmp(&0) {
                    Ordering::Greater => {
                        self.lrotate(rt, t);
                        *l = self.llink(*l, t);
                        rt = self.right(t)?;
                    }
                    Ordering::Equal => {
                        *l = self.llink(*l, rt);
                        return Some(t);
                    }
                    Ordering::Less => {
                        *r = self.rlink(*r, t);
                        *l = self.llink(*l, rt);
                        rt = self.left(t)?;
                    }
                }
            }
        }
    }

    /// `dttree(dt, obj, type)`: returns the object (with its key) that the operation yields.
    pub(crate) fn dttree(&mut self, arg: Option<DtArg<O, K>>, type_: i32) -> Option<(O, K)> {
        self.type_ &= !DT_FOUND;
        let mut root = self.data.here;
        let Some(arg) = arg else {
            let root = root?;
            if type_ & (DT_CLEAR | DT_FIRST | DT_LAST) == 0 {
                return None;
            }
            return self.dttree_without_object(root, type_);
        };

        // The header's `right` is the LEFT tree and its `left` the RIGHT tree.
        let link = LinkId::HEADER;
        self.set_left(link, None);
        self.set_right(link, None);
        let mut l = link;
        let mut r = link;
        let mut obj = arg.obj;

        if self.meth == DT_OBAG && type_ & (DT_DELETE | DT_DETACH) != 0 {
            unimplemented!("dttree: deleting from a bag");
        }
        let mut do_search = false;
        if type_ & (DT_MATCH | DT_SEARCH | DT_INSERT | DT_ATTACH) != 0 {
            do_search = root.is_some();
        } else if type_ & DT_RENEW != 0 {
            unimplemented!("dttree: DT_RENEW");
        } else if let Some(rt) = root {
            do_search = Some(self.obj(rt).0) != obj;
        }
        let key = arg.key;

        if do_search {
            root = self.splay_search(&key, root.expect("searching an empty tree"), &mut l, &mut r);
        }

        let outcome = if let Some(rt) = root {
            // Found it; isolate it.
            self.type_ |= DT_FOUND;
            self.set_right(l, self.left(rt));
            self.set_left(r, self.right(rt));
            if type_ & (DT_SEARCH | DT_MATCH) != 0 {
                Outcome::HasRoot(rt)
            } else if type_ & DT_NEXT != 0 {
                self.set_left(rt, self.right(link));
                self.set_right(rt, None);
                self.set_right(link, Some(rt));
                self.dt_next(link)
            } else if type_ & DT_PREV != 0 {
                unimplemented!("dttree: DT_PREV");
            } else if type_ & (DT_DELETE | DT_DETACH) != 0 {
                obj = Some(self.obj(rt).0);
                self.data.size -= 1;
                assert!(self.data.size >= 0, "dttree: negative size");
                Outcome::NoRoot
            } else if type_ & (DT_INSERT | DT_ATTACH) != 0 {
                if self.meth & DT_OSET != 0 {
                    Outcome::HasRoot(rt)
                } else {
                    self.set_left(rt, None);
                    self.set_right(rt, self.left(link));
                    self.set_left(link, Some(rt));
                    self.dt_insert(obj, key.clone())
                }
            } else {
                unimplemented!("dttree: DT_RENEW of a duplicate");
            }
        } else {
            // Not found; finish up the LEFT and RIGHT trees.
            self.set_left(r, None);
            self.set_right(l, None);
            if type_ & DT_NEXT != 0 {
                self.dt_next(link)
            } else if type_ & DT_PREV != 0 {
                unimplemented!("dttree: DT_PREV");
            } else if type_ & (DT_SEARCH | DT_MATCH) != 0 {
                Outcome::NoRoot
            } else if type_ & (DT_INSERT | DT_ATTACH) != 0 {
                self.dt_insert(obj, key.clone())
            } else {
                unimplemented!("dttree: deleting an object that is not in the dictionary");
            }
        };

        match outcome {
            Outcome::HasRoot(rt) => {
                self.set_left(rt, self.right(link));
                self.set_right(rt, self.left(link));
                if self.meth & DT_OBAG != 0 && type_ & (DT_SEARCH | DT_MATCH) != 0 {
                    unimplemented!("dttree: searching a bag");
                }
                self.data.here = Some(rt);
                Some(self.obj(rt).clone())
            }
            Outcome::NoRoot => {
                let mut r = r;
                while let Some(t) = self.left(r) {
                    r = t;
                }
                self.set_left(r, self.right(link));
                self.data.here = self.left(link);
                if type_ & DT_DELETE != 0 {
                    obj.map(|o| (o, key))
                } else {
                    None
                }
            }
        }
    }

    /// The `obj == NULL` operations: `DT_CLEAR`, `DT_FIRST` and `DT_LAST`.
    fn dttree_without_object(&mut self, mut root: LinkId, type_: i32) -> Option<(O, K)> {
        if type_ & DT_CLEAR != 0 {
            // No discipline that reaches here frees objects, so the links are just dropped.
            self.data.size = 0;
            self.data.here = None;
            return None;
        }
        if type_ & DT_LAST != 0 {
            while let Some(t) = self.right(root) {
                self.lrotate(root, t);
                root = t;
            }
        } else {
            while let Some(t) = self.left(root) {
                self.rrotate(root, t);
                root = t;
            }
        }
        self.data.here = Some(root);
        Some(self.obj(root).clone())
    }

    /// `dt_next:` the smallest element of the RIGHT tree becomes the root.
    fn dt_next(&mut self, link: LinkId) -> Outcome {
        let Some(mut root) = self.left(link) else {
            return Outcome::NoRoot;
        };
        while let Some(t) = self.left(root) {
            self.rrotate(root, t);
            root = t;
        }
        self.set_left(link, self.right(root));
        Outcome::HasRoot(root)
    }

    /// `dt_insert:` a new link for `obj` becomes the root.
    fn dt_insert(&mut self, obj: Option<O>, key: K) -> Outcome {
        let obj = obj.expect("dttree: inserting a template");
        let root = self.new_link(obj, key);
        if self.data.size >= 0 {
            self.data.size += 1;
        }
        Outcome::HasRoot(root)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DT_OBAG, DT_OSET, Dt, DtArg};

    fn set(keys: &[i32]) -> Dt<u32, i32> {
        let mut dt = Dt::dtopen(DT_OSET);
        for (obj, &k) in keys.iter().enumerate() {
            dt.dtinsert(obj as u32, k);
        }
        dt
    }

    fn walk(dt: &mut Dt<u32, i32>, keys: &[i32]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut o = dt.dtfirst();
        while let Some(obj) = o {
            out.push(obj);
            o = dt.dtnext(obj, keys[obj as usize]);
        }
        out
    }

    #[test]
    fn ordered_set_iterates_by_key() {
        let keys = [5, 3, 9, 1, 7, 3];
        let mut dt = set(&keys);
        // The second 3 is a duplicate: inserting it returns the first.
        assert_eq!(dt.dtsize_(), 5);
        assert_eq!(walk(&mut dt, &keys), vec![3, 1, 0, 4, 2]);
    }

    #[test]
    fn insert_of_a_duplicate_returns_the_existing_object() {
        let mut dt = set(&[4, 2]);
        assert_eq!(dt.dtinsert(9, 2), Some(1));
    }

    #[test]
    fn search_finds_by_key_or_not_at_all() {
        let keys = [10, 20, 30, 40];
        let mut dt = set(&keys);
        assert_eq!(dt.dtsearch(DtArg::template(30)), Some(2));
        assert_eq!(dt.dtsearch(DtArg::template(25)), None);
        assert_eq!(walk(&mut dt, &keys), vec![0, 1, 2, 3]);
    }

    #[test]
    fn next_of_an_absent_key_is_its_successor() {
        let keys = [10, 20, 30];
        let mut dt = set(&keys);
        assert_eq!(dt.dtnext(99, 15), Some(1));
        assert_eq!(dt.dtnext(99, 30), None);
    }

    #[test]
    fn delete_removes_and_keeps_order() {
        let keys = [8, 4, 12, 2, 6, 10, 14];
        let mut dt = set(&keys);
        assert_eq!(dt.dtdelete(1, 4), Some(1));
        assert_eq!(dt.dtsize_(), 6);
        assert_eq!(walk(&mut dt, &keys), vec![3, 4, 0, 5, 2, 6]);
    }

    /// The orders Java's `Dtobag` produced for these insertions (xlabels' `Hdisc`).
    #[test]
    fn bag_orders_duplicates_like_java() {
        let cases: [(&[i32], &[u32]); 2] = [
            (&[1, 2, 1, 2, 1], &[2, 4, 0, 3, 1]),
            (
                &[5, 3, 5, 9, 1, 5, 3, 7, 5, 2, 9, 9, 1],
                &[12, 4, 9, 6, 1, 2, 5, 8, 0, 7, 11, 10, 3],
            ),
        ];
        for (keys, java) in cases {
            let mut dt = Dt::dtopen(DT_OBAG);
            for (obj, &k) in keys.iter().enumerate() {
                dt.dtinsert(obj as u32, k);
            }
            assert_eq!(dt.dtsize_(), keys.len() as i32);
            assert_eq!(walk(&mut dt, keys), java);
        }
    }

    #[test]
    fn extract_and_restore_move_a_tree_through_the_dictionary() {
        let mut dt: Dt<u32, i32> = Dt::dtopen(DT_OSET);
        let mut sets = [None, None];
        for (obj, (set, key)) in [(0, 5), (1, 3), (0, 1), (1, 9)].into_iter().enumerate() {
            dt.dtrestore(sets[set]);
            dt.dtinsert(obj as u32, key);
            sets[set] = dt.dtextract();
        }
        dt.dtrestore(sets[1]);
        assert_eq!(dt.dtfirst(), Some(1));
        assert_eq!(dt.dtsize_(), 2);
        sets[1] = dt.dtextract();
        assert_eq!(dt.dtfirst(), None);
        dt.dtrestore(sets[0]);
        assert_eq!(dt.dtfirst(), Some(2));
    }
}
