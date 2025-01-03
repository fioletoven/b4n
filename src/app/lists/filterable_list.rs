use std::{
    cmp::Ordering,
    ops::{Index, IndexMut},
    slice::{Iter, IterMut},
};

#[cfg(test)]
#[path = "./filterable_list.tests.rs"]
mod filterable_list_tests;

/// Wrapper for the [`Vec`] type that provides filtered iterators.  
/// It remembers the original list so the filter can be re-applied anytime with different conditions.  
/// Also it can be more efficient for cases where filtering is CPU bound and the filtered iterator is
/// frequently requested (e.g. drawing a fame on the terminal).
pub struct FilterableList<T> {
    items: Vec<T>,
    filtered: Option<Vec<usize>>,
    len: usize,
}

impl<T> FilterableList<T> {
    /// Creates new [`FilterableList<T>`] instance from the [`Vec`] object.
    pub fn from(items: Vec<T>) -> Self {
        let len = items.len();
        Self {
            items,
            filtered: None,
            len,
        }
    }

    /// Filters out the underneath list.  
    /// __Note__: _the filter is cleared out every time the underneath array is modified_
    pub fn filter<F>(&mut self, f: F)
    where
        F: Fn(&T) -> bool,
    {
        let filtered: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_i, x)| f(x))
            .map(|(i, _x)| i)
            .collect();
        self.len = filtered.len();
        self.filtered = Some(filtered);
    }

    /// Clears the current filter.
    #[inline]
    pub fn filter_reset(&mut self) {
        self.filtered = None;
        self.len = self.items.len();
    }

    /// Returns the number of elements in the filtered out list.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the filtered out list contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.  
    /// __Note__: _clears the current filter_
    pub fn insert(&mut self, index: usize, element: T) {
        self.items.insert(index, element);
        self.filter_reset();
    }

    /// Appends an element to the back of a collection.  
    /// __Note__: _clears the current filter_
    pub fn push(&mut self, value: T) {
        self.items.push(value);
        self.filter_reset();
    }

    /// Returns an iterator over the filtered collection.
    pub fn iter(&self) -> FilterableListIterator<'_, T> {
        FilterableListIterator { list: self, index: 0 }
    }

    /// Returns an iterator, over the filtered collection, that allows modifying each value.
    pub fn iter_mut(&mut self) -> FilterableListIteratorMut<'_, T> {
        FilterableListIteratorMut { list: self, index: 0 }
    }

    /// Returns the number of elements in the underneath collection, also referred to as its 'length'.
    #[inline]
    pub fn full_len(&self) -> usize {
        self.items.len()
    }

    /// Sorts the underneath collection with a comparison function, preserving initial order of equal elements.  
    /// __Note__: _clears the current filter_
    pub fn full_sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.items.sort_by(compare);
        self.filter_reset();
    }

    /// Retains only the elements specified by the predicate in the underneath collection.  
    /// __Note__: _clears the current filter_
    pub fn full_retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.items.retain(f);
        self.filter_reset();
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

impl<T> Index<usize> for FilterableList<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if let Some(list) = &self.filtered {
            &self.items[list[index]]
        } else {
            &self.items[index]
        }
    }
}

impl<T> IndexMut<usize> for FilterableList<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if let Some(list) = &mut self.filtered {
            &mut self.items[list[index]]
        } else {
            &mut self.items[index]
        }
    }
}

impl<'a, T> IntoIterator for &'a FilterableList<T> {
    type Item = &'a T;
    type IntoIter = FilterableListIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        FilterableListIterator { list: self, index: 0 }
    }
}

impl<'a, T> IntoIterator for &'a mut FilterableList<T> {
    type Item = &'a mut T;
    type IntoIter = FilterableListIteratorMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        FilterableListIteratorMut { list: self, index: 0 }
    }
}

/// Iterator struct for the [`FilterableList<T>`]
pub struct FilterableListIterator<'a, T> {
    list: &'a FilterableList<T>,
    index: usize,
}

impl<'a, T> Iterator for FilterableListIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if let Some(list) = &self.list.filtered {
            if list.len() == 0 || self.index >= list.len() || list[self.index] >= self.list.items.len() {
                return None;
            }

            let index = list[self.index];
            self.index += 1;

            Some(&self.list.items[index])
        } else {
            if self.list.items.len() == 0 || self.index >= self.list.items.len() {
                return None;
            }

            let index = self.index;
            self.index += 1;

            Some(&self.list.items[index])
        }
    }
}

/// Mutable iterator struct for the [`FilterableList<T>`]
pub struct FilterableListIteratorMut<'a, T> {
    list: &'a mut FilterableList<T>,
    index: usize,
}

impl<'a, T> Iterator for FilterableListIteratorMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        if let Some(list) = &mut self.list.filtered {
            if list.len() == 0 || self.index >= list.len() || list[self.index] >= self.list.items.len() {
                return None;
            }

            let item = unsafe { &mut *(&mut self.list.items[list[self.index]] as *mut T) };
            self.index += 1;

            Some(item)
        } else {
            if self.list.items.len() == 0 || self.index >= self.list.items.len() {
                return None;
            }

            let item = unsafe { &mut *(&mut self.list.items[self.index] as *mut T) };
            self.index += 1;

            Some(item)
        }
    }
}
