use std::collections::hash_map::{Iter, IterMut};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A cache containing data that can be accessed through generated handles
pub struct Cache<T> {
    data: HashMap<Handle<T>, T>,
    prev_index: u64,
}

impl<T> Cache<T> {
    /// Factory method
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            prev_index: 0,
        }
    }

    /// Inserts a value into the cache
    pub fn insert(&mut self, value: T) -> Handle<T> {
        self.prev_index += 1;
        let handle = Handle::new(self.prev_index);
        self.data.insert(handle, value);
        handle
    }

    /// Removes a value from the cache
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        self.data.remove(&handle)
    }

    /// Gets a reference to a value stored within the cache
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.data.get(&handle)
    }

    /// Gets a mutable reference to a value stored within the cache
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.data.get_mut(&handle)
    }

    /// Creates an iterator over references to the handle-value pairs contained in the cache
    pub fn iter(&self) -> Iter<'_, Handle<T>, T> {
        self.data.iter()
    }

    /// Creates an iterator over mutable references to the handle-value pairs contained in the cache
    pub fn iter_mut(&mut self) -> IterMut<'_, Handle<T>, T> {
        self.data.iter_mut()
    }
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A handle pointing to some sort of data within a cache
pub struct Handle<T> {
    index: u64,
    phantom_data: PhantomData<T>,
}

impl<T> Handle<T> {
    fn new(index: u64) -> Self {
        Self {
            index,
            phantom_data: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle {{ index: {} }}", self.index)
    }
}
