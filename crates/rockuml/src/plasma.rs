//! The namespace of a diagram's entities: a tree of named quarks, where a qualified name like `a.b.c` walks
//! down from the root (PlantUML's `plasma` package).

use std::collections::HashMap;

/// Separates the parts of qualified names when the diagram has no separator, so that names never split.
pub(crate) const MAGIC_SEPARATOR: &str = "\u{1}";

/// A quark, by its place in the order quarks were created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct QuarkId(usize);

impl QuarkId {
    /// A plasma creates its root first.
    pub(crate) fn is_root(self) -> bool {
        self.0 == 0
    }
}

/// A named node holding at most one piece of data, `D`, set once.
#[derive(Debug)]
pub(crate) struct Quark<D> {
    parent: Option<QuarkId>,
    name: String,
    data: Option<D>,
    /// In the order they were created, as Java's `LinkedHashMap` keeps them.
    children: Vec<QuarkId>,
    children_by_name: HashMap<String, QuarkId>,
    qualified_name: String,
}

impl<D: Copy> Quark<D> {
    pub(crate) fn get_parent(&self) -> Option<QuarkId> {
        self.parent
    }

    pub(crate) fn get_name(&self) -> &str {
        &self.name
    }

    /// The names from below the root down to this quark, joined by the separator in force when the quark was
    /// created.
    pub(crate) fn get_qualified_name(&self) -> &str {
        &self.qualified_name
    }

    pub(crate) fn get_data(&self) -> Option<D> {
        self.data
    }

    pub(crate) fn get_children(&self) -> &[QuarkId] {
        &self.children
    }

    pub(crate) fn count_children(&self) -> usize {
        self.children.len()
    }
}

/// How many quarks bear a name, and the first of them.
struct PEntry {
    first: QuarkId,
    counter: usize,
}

pub(crate) struct Plasma<D> {
    separator: String,
    /// Every quark, the root first, in creation order.
    quarks: Vec<Quark<D>>,
    /// Only looked up, so Java's `HashMap` order never shows.
    stats: HashMap<String, PEntry>,
}

impl<D: Copy> Default for Plasma<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Copy> Plasma<D> {
    pub(crate) fn new() -> Self {
        let mut plasma = Self {
            separator: MAGIC_SEPARATOR.to_owned(),
            quarks: Vec::new(),
            stats: HashMap::new(),
        };
        plasma.create(None, String::new());
        plasma
    }

    fn create(&mut self, parent: Option<QuarkId>, name: String) -> QuarkId {
        let id = QuarkId(self.quarks.len());
        let qualified_name = match parent {
            Some(parent) if !parent.is_root() => format!(
                "{}{}{}",
                self.quark(parent).qualified_name,
                self.separator,
                name
            ),
            _ => name.clone(),
        };
        self.stats
            .entry(name.clone())
            .and_modify(|entry| entry.counter += 1)
            .or_insert(PEntry {
                first: id,
                counter: 1,
            });
        if let Some(parent) = parent {
            let parent = &mut self.quarks[parent.0];
            parent.children.push(id);
            parent.children_by_name.insert(name.clone(), id);
        }
        self.quarks.push(Quark {
            parent,
            name,
            data: None,
            children: Vec::new(),
            children_by_name: HashMap::new(),
            qualified_name,
        });
        id
    }

    pub(crate) fn root(&self) -> QuarkId {
        QuarkId(0)
    }

    pub(crate) fn quark(&self, id: QuarkId) -> &Quark<D> {
        &self.quarks[id.0]
    }

    pub(crate) fn get_separator(&self) -> &str {
        &self.separator
    }

    /// `None` keeps names whole.
    pub(crate) fn set_separator(&mut self, separator: Option<&str>) {
        self.separator = separator.unwrap_or(MAGIC_SEPARATOR).to_owned();
    }

    pub(crate) fn has_separator(&self) -> bool {
        self.separator != MAGIC_SEPARATOR
    }

    /// Every quark, the root first, in creation order.
    pub(crate) fn quarks(&self) -> impl Iterator<Item = QuarkId> + use<D> {
        (0..self.quarks.len()).map(QuarkId)
    }

    pub(crate) fn first_with_name(&self, name: &str) -> Option<QuarkId> {
        self.stats.get(name).map(|entry| entry.first)
    }

    pub(crate) fn count_by_name(&self, name: &str) -> usize {
        self.stats.get(name).map_or(0, |entry| entry.counter)
    }

    /// # Panics
    ///
    /// If the quark already holds data: PlantUML sets it once.
    pub(crate) fn set_data(&mut self, id: QuarkId, data: D) {
        let quark = &mut self.quarks[id.0];
        assert!(
            quark.data.is_none(),
            "quark {} already has data",
            quark.qualified_name
        );
        quark.data = Some(data);
    }

    /// The child named `name`, a single part, if it exists.
    pub(crate) fn child_if_exists(&self, id: QuarkId, name: &str) -> Option<QuarkId> {
        self.quark(id).children_by_name.get(name).copied()
    }

    /// The quark `full` names below `id`, created with any missing parent. Leading and trailing separators
    /// are ignored.
    pub(crate) fn child(&mut self, id: QuarkId, full: &str) -> QuarkId {
        if !self.has_separator() {
            return self.get_direct_child(id, full);
        }
        let separator = self.separator.clone();
        let mut full = clean(full, &separator);
        let mut current = id;
        while let Some(idx) = full.find(&separator) {
            current = self.get_direct_child(current, &full[..idx]);
            full = clean(&full[idx + separator.len()..], &separator);
        }
        self.get_direct_child(current, full)
    }

    fn get_direct_child(&mut self, id: QuarkId, name: &str) -> QuarkId {
        match self.child_if_exists(id, name) {
            Some(child) => child,
            None => self.create(Some(id), name.to_owned()),
        }
    }
}

fn clean<'a>(mut full: &'a str, separator: &str) -> &'a str {
    while let Some(rest) = full.strip_prefix(separator) {
        full = rest;
    }
    while let Some(rest) = full.strip_suffix(separator) {
        full = rest;
    }
    full
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(plasma: &Plasma<u8>) -> Vec<String> {
        plasma
            .quarks()
            .map(|id| plasma.quark(id).get_qualified_name().to_owned())
            .collect()
    }

    #[test]
    fn without_separator_names_stay_whole() {
        let mut plasma = Plasma::<u8>::new();
        let root = plasma.root();
        let quark = plasma.child(root, "a.b");
        assert_eq!(plasma.quark(quark).get_name(), "a.b");
        assert_eq!(plasma.quark(quark).get_parent(), Some(root));
        assert_eq!(plasma.child(root, "a.b"), quark);
    }

    #[test]
    fn a_separator_walks_down_creating_parents() {
        let mut plasma = Plasma::<u8>::new();
        plasma.set_separator(Some("."));
        let root = plasma.root();
        let c = plasma.child(root, ".a.b.c.");
        assert_eq!(names(&plasma), ["", "a", "a.b", "a.b.c"]);
        let a = plasma.child_if_exists(root, "a").unwrap();
        assert_eq!(plasma.child(a, "b.c"), c);
        assert_eq!(plasma.quark(a).get_children().len(), 1);
    }

    #[test]
    fn qualified_names_use_the_separator_in_force_at_creation() {
        let mut plasma = Plasma::<u8>::new();
        plasma.set_separator(Some("::"));
        let root = plasma.root();
        plasma.child(root, "a::b");
        plasma.set_separator(None);
        let a = plasma.child_if_exists(root, "a").unwrap();
        plasma.child(a, "c::d");
        assert_eq!(names(&plasma), ["", "a", "a::b", "a\u{1}c::d"]);
    }

    #[test]
    fn names_are_counted_with_their_first_quark() {
        let mut plasma = Plasma::<u8>::new();
        plasma.set_separator(Some("."));
        let root = plasma.root();
        let first = plasma.child(root, "x.leaf");
        plasma.child(root, "y.leaf");
        assert_eq!(plasma.count_by_name("leaf"), 2);
        assert_eq!(plasma.first_with_name("leaf"), Some(first));
        assert_eq!(plasma.count_by_name("z"), 0);
        assert_eq!(plasma.first_with_name("z"), None);
        assert_eq!(plasma.count_by_name(""), 1);
    }

    #[test]
    fn children_keep_their_creation_order() {
        let mut plasma = Plasma::<u8>::new();
        let root = plasma.root();
        let ids: Vec<QuarkId> = ["c", "a", "b"]
            .iter()
            .map(|name| plasma.child(root, name))
            .collect();
        assert_eq!(plasma.quark(root).get_children(), ids.as_slice());
        plasma.set_data(ids[1], 7);
        assert_eq!(plasma.quark(ids[1]).get_data(), Some(7));
        assert_eq!(plasma.quark(ids[0]).get_data(), None);
        assert!(root.is_root() && !ids[0].is_root());
    }
}
