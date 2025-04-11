use std::{
    cmp::Ordering,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice::{Iter, IterMut},
};

use super::{FilterContext, Filterable};

#[cfg(test)]
#[path = "./filterable_list.tests.rs"]
mod filterable_list_tests;

/// Wrapper for the [`Vec`] type that provides filtered iterators.  
/// It remembers the original list so the filter can be re-applied anytime with different conditions.  
/// Also it can be more efficient for cases where filtering is CPU bound and the filtered iterator is
/// frequently requested (e.g. drawing a fame on the terminal).
pub struct FilterableList<T: Filterable<Fc>, Fc: FilterContext> {
    items: Vec<T>,
    filtered: Option<Vec<usize>>,
    _marker: PhantomData<Fc>,
}

impl<T: Filterable<Fc>, Fc: FilterContext> FilterableList<T, Fc> {
    /// Creates new [`FilterableList<T, Fc>`] instance from the [`Vec`] object.
    pub fn from(items: Vec<T>) -> Self {
        Self {
            items,
            filtered: None,
            _marker: PhantomData,
        }
    }

    /// Clears the [`FilterableList<T, Fc>`], removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
        self.filter_reset();
    }

    /// Removes and returns the element at position `index` within the filtered out list.  
    /// **Note** that this clears the current filter.
    pub fn remove(&mut self, index: usize) -> T {
        if let Some(list) = &self.filtered {
            let index = list[index];
            self.filter_reset();
            self.items.remove(index)
        } else {
            self.filter_reset();
            self.items.remove(index)
        }
    }

    /// Filters out the underneath list using `context` for that.  
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

    /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.  
    /// **Note** that this clears the current filter.
    pub fn insert(&mut self, index: usize, element: T) {
        self.items.insert(index, element);
        self.filter_reset();
    }

    /// Appends an element to the back of a collection.  
    /// **Note** that this clears the current filter.
    pub fn push(&mut self, value: T) {
        self.items.push(value);
        self.filter_reset();
    }

    /// Returns an iterator over the filtered collection.
    pub fn iter(&self) -> FilterableListIterator<'_, T, Fc> {
        FilterableListIterator { list: self, index: 0 }
    }

    /// Returns an iterator, over the filtered collection, that allows modifying each value.
    pub fn iter_mut(&mut self) -> FilterableListIteratorMut<'_, T, Fc> {
        FilterableListIteratorMut { list: self, index: 0 }
    }

    /// Returns the number of elements in the underneath collection, also referred to as its 'length'.
    #[inline]
    pub fn full_len(&self) -> usize {
        self.items.len()
    }

    /// Sorts the underneath collection with a comparison function, preserving initial order of equal elements.  
    /// **Note** that this clears the current filter.
    pub fn full_sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.items.sort_by(compare);
        self.filter_reset();
    }

    /// Retains only the elements specified by the predicate in the underneath collection.  
    /// **Note** that this clears the current filter.
    pub fn full_retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.items.retain(f);
        self.filter_reset();
    }

    /// Removes and returns the element at position `index` within the underneath collection.  
    /// **Note** that this clears the current filter.
    pub fn full_remove(&mut self, index: usize) -> T {
        self.filter_reset();
        self.items.remove(index)
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
        if let Some(list) = &mut self.filtered {
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
        FilterableListIterator { list: self, index: 0 }
    }
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> IntoIterator for &'a mut FilterableList<T, Fc> {
    type Item = &'a mut T;
    type IntoIter = FilterableListIteratorMut<'a, T, Fc>;

    fn into_iter(self) -> Self::IntoIter {
        FilterableListIteratorMut { list: self, index: 0 }
    }
}

/// Iterator struct for the [`FilterableList<T, Fc>`]
pub struct FilterableListIterator<'a, T: Filterable<Fc>, Fc: FilterContext> {
    list: &'a FilterableList<T, Fc>,
    index: usize,
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> Iterator for FilterableListIterator<'a, T, Fc> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if let Some(list) = &self.list.filtered {
            if list.is_empty() || self.index >= list.len() || list[self.index] >= self.list.items.len() {
                return None;
            }

            let index = list[self.index];
            self.index += 1;

            Some(&self.list.items[index])
        } else {
            if self.list.items.is_empty() || self.index >= self.list.items.len() {
                return None;
            }

            let index = self.index;
            self.index += 1;

            Some(&self.list.items[index])
        }
    }
}

/// Mutable iterator struct for the [`FilterableList<T, Fc>`]
pub struct FilterableListIteratorMut<'a, T: Filterable<Fc>, Fc: FilterContext> {
    list: &'a mut FilterableList<T, Fc>,
    index: usize,
}

impl<'a, T: Filterable<Fc>, Fc: FilterContext> Iterator for FilterableListIteratorMut<'a, T, Fc> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        if let Some(list) = &mut self.list.filtered {
            if list.is_empty() || self.index >= list.len() || list[self.index] >= self.list.items.len() {
                return None;
            }

            let item = unsafe { &mut *(&mut self.list.items[list[self.index]] as *mut T) };
            self.index += 1;

            Some(item)
        } else {
            if self.list.items.is_empty() || self.index >= self.list.items.len() {
                return None;
            }

            let item = unsafe { &mut *(&mut self.list.items[self.index] as *mut T) };
            self.index += 1;

            Some(item)
        }
    }
}
