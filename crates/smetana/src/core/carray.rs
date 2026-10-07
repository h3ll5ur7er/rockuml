//! `CArray` and `CArrayOfStar`: C arrays as Smetana emulates them, a buffer plus an offset.
//!
//! Smetana's dot code aliases arrays the way C does, and the layout depends on it: a cluster's rank vectors point
//! into the root's (`GD_rank(clust)[r].v = GD_rank(root)[r].v + pos`), `GD_rank(g)` is moved one element forward so
//! that `rank[-1]` exists, and `REALLOC` hands back the same array when it is big enough. So an array is a
//! [`CArray`] handle, a buffer id plus an offset that may make indices negative, into a [`CArrays`] store. Copying
//! a handle aliases the buffer like copying a C pointer.
//!
//! `CArrayOfStar<ST_Agnode_s>` becomes `CArray<Option<NodeId>>`; a `CArray` of structs becomes a `CArray` of the
//! (`Copy`) struct. Java's `REALLOC__` of a struct array shares the element objects between the old and the grown
//! array; here growth copies them, which differs only if someone still writes through the old handle afterwards,
//! and no Smetana code does.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A pointer into a [`CArrays`] buffer: `CArray<O>` and `CArrayOfStar<O>` of the Java tree.
pub struct CArray<T> {
    buf: u32,
    offset: i32,
    elem: PhantomData<fn() -> T>,
}

impl<T> Clone for CArray<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for CArray<T> {}

impl<T> PartialEq for CArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.buf == other.buf && self.offset == other.offset
    }
}

impl<T> Eq for CArray<T> {}

impl<T> fmt::Debug for CArray<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CArray(buf {} + {})", self.buf, self.offset)
    }
}

impl<T> CArray<T> {
    /// Pointer arithmetic: the array starting `delta` elements further.
    #[must_use]
    pub fn plus_(self, delta: i32) -> Self {
        Self {
            buf: self.buf,
            offset: self.offset + delta,
            elem: PhantomData,
        }
    }

    /// Pointer difference; both must point into the same buffer, as Java checks.
    pub fn minus_(self, other: Self) -> i32 {
        assert_eq!(self.buf, other.buf, "pointer difference across arrays");
        self.offset - other.offset
    }

    /// `comparePointer_`, which Java also only allows within one buffer.
    pub fn compare_pointer_(self, other: Self) -> i32 {
        self.minus_(other)
    }

    /// The element at `i`, for indexing a [`CArrays`]: `arrays[a.at(i)]`.
    pub fn at(self, i: i32) -> Elem<T> {
        Elem { array: self, i }
    }
}

/// One element of a [`CArray`], an index into its [`CArrays`] store.
pub struct Elem<T> {
    array: CArray<T>,
    i: i32,
}

/// The buffers of all arrays of one element type.
pub struct CArrays<T> {
    bufs: Vec<Vec<T>>,
}

impl<T> Default for CArrays<T> {
    fn default() -> Self {
        Self { bufs: Vec::new() }
    }
}

impl<T: Clone + Default> CArrays<T> {
    /// `ALLOC__` / `ALLOC`: a new zero-initialised (`Default`) array.
    #[allow(non_snake_case)]
    pub fn ALLOC(&mut self, size: i32) -> CArray<T> {
        let size = usize::try_from(size).expect("negative array size");
        self.bufs.push(vec![T::default(); size]);
        CArray {
            buf: u32::try_from(self.bufs.len() - 1).expect("too many arrays"),
            offset: 0,
            elem: PhantomData,
        }
    }

    /// `REALLOC__` / `REALLOC`: `old` itself when its buffer holds `size` elements (its offset is ignored, as in
    /// Java), otherwise a new array holding `old`'s elements followed by default ones.
    #[allow(non_snake_case)]
    pub fn REALLOC(&mut self, size: i32, old: Option<CArray<T>>) -> CArray<T> {
        let Some(old) = old else {
            return self.ALLOC(size);
        };
        let wanted = usize::try_from(size).expect("negative array size");
        let current = &self.bufs[old.buf as usize];
        if wanted <= current.len() {
            return old;
        }
        assert_eq!(old.offset, 0, "REALLOC of an offset array");
        let mut grown = current.clone();
        grown.resize(wanted, T::default());
        self.bufs.push(grown);
        CArray {
            buf: u32::try_from(self.bufs.len() - 1).expect("too many arrays"),
            offset: 0,
            elem: PhantomData,
        }
    }
}

impl<T: Copy> CArrays<T> {
    /// `get__` / `get_`: the element at `i` (which may be negative relative to the array's offset).
    pub fn get(&self, array: CArray<T>, i: i32) -> T {
        self[array.at(i)]
    }

    /// `set_`.
    pub fn set(&mut self, array: CArray<T>, i: i32, value: T) {
        self[array.at(i)] = value;
    }

    /// `CArrayOfStar._swap`, offset-aware as PlantUML patched it.
    pub fn swap(&mut self, array: CArray<T>, i: i32, j: i32) {
        let (i, j) = (Self::slot(array, i), Self::slot(array, j));
        self.bufs[array.buf as usize].swap(i, j);
    }

    /// The `n` elements from `array[0]`, for algorithms (sorts) that work on a slice.
    pub fn to_vec(&self, array: CArray<T>, n: i32) -> Vec<T> {
        (0..n).map(|i| self.get(array, i)).collect()
    }

    /// Writes `values` back to `array[0..]`.
    pub fn copy_from(&mut self, array: CArray<T>, values: &[T]) {
        for (i, v) in values.iter().enumerate() {
            self.set(array, i32::try_from(i).expect("array too long"), *v);
        }
    }
}

impl<T> CArrays<T> {
    fn slot(array: CArray<T>, i: i32) -> usize {
        usize::try_from(array.offset + i).expect("index before the start of the array")
    }
}

impl<T> Index<Elem<T>> for CArrays<T> {
    type Output = T;

    fn index(&self, e: Elem<T>) -> &T {
        &self.bufs[e.array.buf as usize][Self::slot(e.array, e.i)]
    }
}

impl<T> IndexMut<Elem<T>> for CArrays<T> {
    fn index_mut(&mut self, e: Elem<T>) -> &mut T {
        &mut self.bufs[e.array.buf as usize][Self::slot(e.array, e.i)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_alias_and_allow_negative_indices() {
        let mut arrays = CArrays::<i32>::default();
        let base = arrays.ALLOC(5);
        let shifted = base.plus_(1);
        arrays.set(shifted, -1, 7);
        assert_eq!(arrays.get(base, 0), 7);
        arrays.set(base, 3, 9);
        assert_eq!(arrays[shifted.at(2)], 9);
        assert_eq!(shifted.minus_(base), 1);
    }

    #[test]
    fn realloc_returns_the_same_array_when_big_enough() {
        let mut arrays = CArrays::<i32>::default();
        let a = arrays.ALLOC(4);
        arrays.set(a, 1, 5);
        assert_eq!(arrays.REALLOC(3, Some(a)), a);
        assert_eq!(arrays.REALLOC(4, Some(a.plus_(1))), a.plus_(1));
        let grown = arrays.REALLOC(6, Some(a));
        assert_ne!(grown, a);
        assert_eq!(arrays.to_vec(grown, 6), vec![0, 5, 0, 0, 0, 0]);
    }

    #[test]
    fn swap_respects_the_offset() {
        let mut arrays = CArrays::<i32>::default();
        let a = arrays.ALLOC(4);
        arrays.copy_from(a, &[1, 2, 3, 4]);
        arrays.swap(a.plus_(2), 0, 1);
        assert_eq!(arrays.to_vec(a, 4), vec![1, 2, 4, 3]);
    }

    #[test]
    #[should_panic(expected = "pointer difference across arrays")]
    fn pointer_difference_needs_one_buffer() {
        let mut arrays = CArrays::<i32>::default();
        let a = arrays.ALLOC(1);
        let b = arrays.ALLOC(1);
        let _ = a.minus_(b);
    }
}
