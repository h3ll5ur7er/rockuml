//! Typed handles that replace Java's object references, and the arenas they index.
//!
//! A handle is `Copy` and compares by identity like a Java reference (`==`). Its numeric order is allocation order,
//! which ported code must not rely on: Smetana orders objects by their tag's `seq` or `id`, never by address.
//! Nothing is ever freed: a layout allocates, then the whole [`Globals`](super::Globals) is dropped.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A handle into an [`Arena`].
pub trait ArenaId: Copy {
    fn from_index(index: usize) -> Self;
    fn index(self) -> usize;
}

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u32);

        impl ArenaId for $name {
            fn from_index(index: usize) -> Self {
                Self(u32::try_from(index).expect("arena overflow"))
            }

            fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

id_type!(
    /// A root graph or a subgraph (`ST_Agraph_s`).
    GraphId
);
id_type!(
    /// A node (`ST_Agnode_s`), real or virtual.
    NodeId
);
id_type!(
    /// A node's membership in one (sub)graph (`ST_Agsubnode_s`).
    SubnodeId
);
id_type!(
    /// An attribute declaration (`ST_Agsym_s`).
    SymId
);
id_type!(
    /// An interned string (`ST_refstr_t` and its `CString`).
    StrId
);
id_type!(
    /// The resources a root graph shares with its subgraphs (`ST_Agclos_s`).
    ClosId
);
id_type!(
    /// A `textlabel_t`.
    TextlabelId
);
id_type!(
    /// An edge's `splines`.
    SplinesId
);
id_type!(
    /// A node's `polygon_t` shape information.
    PolygonId
);
id_type!(
    /// A record node's `field_t`.
    FieldId
);
id_type!(
    /// A rank's flat-edge `adjmatrix_t`.
    AdjmatrixId
);
id_type!(
    /// An entry of `Globals::Shapes` (`shape_desc`).
    ShapeDescId
);
id_type!(
    /// An attribute dictionary (one of an `Agdatadict_s`'s `dict_n`/`dict_e`/`dict_g`).
    DictId
);

/// One half of an edge (`ST_Agedge_s`). Edges always come in `ST_Agedgepair_s` pairs, an out-edge (`AGOUTEDGE`,
/// whose `node` is the head) and an in-edge (`AGINEDGE`, whose `node` is the tail). The id encodes the pair and the
/// half, so `AGOPP`, `AGMKOUT` and `AGMKIN` are bit operations; the pair's index is its [`Arena`] index.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(u32);

impl EdgeId {
    pub(crate) fn out_of_pair(pair: usize) -> Self {
        Self(u32::try_from(pair * 2).expect("arena overflow"))
    }

    pub(crate) fn pair(self) -> usize {
        (self.0 >> 1) as usize
    }

    pub(crate) fn is_in_half(self) -> bool {
        self.0 & 1 == 1
    }

    /// The other half of the pair, whatever the halves' tags say.
    pub(crate) fn flip(self) -> Self {
        Self(self.0 ^ 1)
    }
}

impl fmt::Debug for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let half = if self.is_in_half() { "in" } else { "out" };
        write!(f, "EdgeId({}.{half})", self.pair())
    }
}

/// Storage for one kind of object, indexed by its handle type.
pub struct Arena<I, T> {
    items: Vec<T>,
    id: PhantomData<fn() -> I>,
}

impl<I: ArenaId, T> Arena<I, T> {
    pub fn push(&mut self, item: T) -> I {
        self.items.push(item);
        I::from_index(self.items.len() - 1)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The handles in allocation order.
    pub fn ids(&self) -> impl Iterator<Item = I> + use<I, T> {
        (0..self.items.len()).map(I::from_index)
    }
}

impl<I, T> Default for Arena<I, T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            id: PhantomData,
        }
    }
}

impl<I: ArenaId, T> Index<I> for Arena<I, T> {
    type Output = T;

    fn index(&self, id: I) -> &T {
        &self.items[id.index()]
    }
}

impl<I: ArenaId, T> IndexMut<I> for Arena<I, T> {
    fn index_mut(&mut self, id: I) -> &mut T {
        &mut self.items[id.index()]
    }
}
