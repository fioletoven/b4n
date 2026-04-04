use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::slice::{Iter, IterMut};

use super::{FilterContext, Filterable};

#[cfg(test)]
#[path = "./filterable_list.tests.rs"]
pub mod filterable_list_tests;

/// Wrapper for the [`Vec`] type that provides filtered iterators.\
/// It remembers the original list so the filter can be re-applied anytime with different conditions.\
/// Also it can be more efficient for cases where filtering is CPU bound and the filtered iterator is
/// frequently requested (e.g. drawing a fame on the terminal).
pub struct FilterableList<T, Fc> {
    items: Vec<T>,
    filtered: Option<Vec<usize>>,
    _marker: PhantomData<Fc>,
}

impl<T, Fc> Default for FilterableList<T, Fc> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            filtered: None,
            _marker: PhantomData,
        }
    }
}

impl<T: Filterable<Fc>, Fc: FilterContext> From<Vec<T>> for FilterableList<T, Fc> {
    fn from(value: Vec<T>) -> Self {
        Self {
            items: value,
            filtered: None,
            _marker: PhantomData,
        }
    }
}

impl<T: Filterable<Fc>, Fc: FilterContext> FilterableList<T, Fc> {
    /// Clears the [`FilterableList<T, Fc>`], removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
        self.filter_reset();
    }

    /// Removes and returns the element at position `index` within the filtered out list.\
    /// **Note** that this clears the current filter.
    pub fn remove(&mut self, index: usize) -> T {
        let actual_index = match &self.filtered {
            Some(list) => list[index],
            None => index,
        };
        self.filter_reset();
        self.items.remove(actual_index)
    }

    /// Filters out the underneath list using `context` for that.\
    /// **Note** that the filter is cleared out every time the underneath array is modified.
    pub fn filter(&mut self, context: &mut Fc) {
        let filtered: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_i, x)| x.is_matching(context))
            .map(|(i, _x)| i)
            .collect();
        self.filtered = Some(filtered);
    }

    /// Clears the current filter.
    #[inline]
    pub fn filter_reset(&mut self) {
        self.filtered = None;
    }

    /// Returns the number of elements in the filtered out list.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.filtered {
            Some(filtered) => filtered.len(),
            None => self.items.len(),
        }
    }

    /// Returns `true` if the filtered out list contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.\
    /// **Note** that this clears the current filter.
    pub fn insert(&mut self, index: usize, element: T) {
        self.items.insert(index, element);
        self.filter_reset();
    }

    /// Appends an element to the back of a collection.\
    /// **Note** that this clears the current filter.
    pub fn push(&mut self, value: T) {
        self.items.push(value);
        self.filter_reset();
    }

    /// Returns an iterator over the filtered collection.
    pub fn iter(&self) -> FilterableListIterator<'_, T, Fc> {
        let inner = match self.filtered.as_deref() {
            None => IterInner::All(self.items.iter()),
            Some(indices) => IterInner::Filtered {
                items: &self.items,
                indices: indices.iter(),
            },
        };

        FilterableListIterator {
            inner,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator, over the filtered collection, that allows modifying each value.
    pub fn iter_mut(&mut self) -> FilterableListIteratorMut<'_, T, Fc> {
        let inner = match self.filtered.as_deref() {
            None => IterMutInner::All(self.items.iter_mut()),
            Some(indices) => IterMutInner::Filtered {
                ptr: self.items.as_mut_ptr(),
                len: self.items.len(),
                indices: indices.iter(),
                _marker: PhantomData,
            },
        };

        FilterableListIteratorMut {
            inner,
            _marker: PhantomData,
        }
    }

    /// Returns the number of elements in the underneath collection, also referred to as its 'length'.
    #[inline]
    pub fn full_len(&self) -> usize {
        self.items.len()
    }

    /// Sorts the underneath collection with a comparison function, preserving initial order of equal elements.\
    /// **Note** that this clears the current filter.
    pub fn full_sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.items.sort_by(compare);
        self.filter_reset();
    }

    /// Retains only the elements specified by the predicate in the underneath collection.\
    /// **Note** that this clears the current filter.
    pub fn full_retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.items.retain(f);
        self.filter_reset();
    }

    /// Removes and returns the element at position `index` within the underneath collection.\
    /// **Note** that this clears the current filter.
    pub fn full_remove(&mut self, index: usize) -> T {
        self.filter_reset();
        self.items.remove(index)
    }

    /// Replaces value at position `index`.\
    /// **Note** that this clears the current filter.
    pub fn full_replace(&mut self, index: usize, value: T) -> T {
        self.filter_reset();
        std::mem::replace(&mut self.items[index], value)
    }

    /// Returns an iterator over the underneath collection.
    pub fn full_iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }

    /// Returns an iterator, over the underneath collection, that allows modifying each value.
    pub fn full_iter_mut(&mut self) -> IterMut<'_, T> {
        self.items.iter_mut()
    }
}

impl<T: Filterable<Fc>, Fc: FilterContext> Index<usize> for FilterableList<T, Fc> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if let Some(list) = &self.filtered {
            &self.items[list[index]]
        } else {
            &self.items[index]
        }
    }
}

impl<T: Filterable<Fc>, Fc: FilterContext> IndexMut<usize> for FilterableList<T, Fc> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if let Some(list) = &self.filtered {
            &mut self.items[list[index]]
        } else {
            &mut self.items[index]
        }
    }
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> IntoIterator for &'a FilterableList<T, Fc> {
    type Item = &'a T;
    type IntoIter = FilterableListIterator<'a, T, Fc>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> IntoIterator for &'a mut FilterableList<T, Fc> {
    type Item = &'a mut T;
    type IntoIter = FilterableListIteratorMut<'a, T, Fc>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator struct for the [`FilterableList<T, Fc>`]
pub struct FilterableListIterator<'a, T, Fc> {
    inner: IterInner<'a, T>,
    _marker: PhantomData<Fc>,
}

enum IterInner<'a, T> {
    All(Iter<'a, T>),
    Filtered {
        items: &'a [T],
        indices: Iter<'a, usize>,
    },
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> Iterator for FilterableListIterator<'a, T, Fc> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match &mut self.inner {
            IterInner::All(iter) => iter.next(),
            IterInner::Filtered { items, indices } => {
                let &idx = indices.next()?;
                Some(&items[idx])
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            IterInner::All(iter) => iter.size_hint(),
            IterInner::Filtered { indices, .. } => indices.size_hint(),
        }
    }
}

impl<T: Filterable<Fc>, Fc: FilterContext> ExactSizeIterator for FilterableListIterator<'_, T, Fc> {}

/// Mutable iterator struct for the [`FilterableList<T, Fc>`]
pub struct FilterableListIteratorMut<'a, T, Fc> {
    inner: IterMutInner<'a, T>,
    _marker: PhantomData<Fc>,
}

enum IterMutInner<'a, T> {
    All(IterMut<'a, T>),
    Filtered {
        ptr: *mut T,
        len: usize,
        indices: Iter<'a, usize>,
        _marker: PhantomData<&'a mut [T]>,
    },
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> Iterator for FilterableListIteratorMut<'a, T, Fc> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        match &mut self.inner {
            IterMutInner::All(iter) => iter.next(),
            IterMutInner::Filtered { ptr, len, indices, .. } => {
                let &idx = indices.next()?;
                assert!(idx < *len, "filtered index out of bounds");

                // SAFETY: Caller guarantees `filtered` contains no duplicate indices, \
                // so each element is yielded at most once, therefore no aliasing &mut T.
                Some(unsafe { &mut *ptr.add(idx) })
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            IterMutInner::All(iter) => iter.size_hint(),
            IterMutInner::Filtered { indices, .. } => indices.size_hint(),
        }
    }
}

impl<T: Filterable<Fc>, Fc: FilterContext> ExactSizeIterator for FilterableListIteratorMut<'_, T, Fc> {}

// SAFETY: The filtered variant conceptually holds &'a mut [T],
// which is Send when T: Send and Sync when T: Sync.
unsafe impl<T: Send, Fc> Send for FilterableListIteratorMut<'_, T, Fc> {}
unsafe impl<T: Sync, Fc> Sync for FilterableListIteratorMut<'_, T, Fc> {}
