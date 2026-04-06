//! Arena allocator — Module 2
//! Separate typed arenas per object kind for cache locality.
//! HeapRef is always a u32 index — compactness + no aliasing.

pub struct Arena<T> {
    slots:     Vec<Option<T>>,
    free_list: Vec<u32>,
}

impl<T> Arena<T> {
    pub fn new() -> Self { Self { slots: Vec::new(), free_list: Vec::new() } }

    /// Allocate a slot — O(1) on fast path (free list non-empty).
    pub fn alloc(&mut self, val: T) -> super::HeapRef {
        if let Some(idx) = self.free_list.pop() {
            self.slots[idx as usize] = Some(val);
            super::HeapRef(idx)
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(Some(val));
            super::HeapRef(idx)
        }
    }

    pub fn get(&self, r: super::HeapRef) -> Option<&T> {
        self.slots.get(r.0 as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, r: super::HeapRef) -> Option<&mut T> {
        self.slots.get_mut(r.0 as usize)?.as_mut()
    }

    /// Free a slot — called by GC sweep phase.
    pub fn free(&mut self, r: super::HeapRef) {
        if let Some(slot) = self.slots.get_mut(r.0 as usize) {
            *slot = None;
            self.free_list.push(r.0);
        }
    }

    pub fn len(&self) -> usize { self.slots.len() }

    pub fn iter_occupied(&self) -> impl Iterator<Item = (super::HeapRef, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.as_ref().map(|v| (super::HeapRef(i as u32), v))
        })
    }
}

impl<T> Default for Arena<T> { fn default() -> Self { Self::new() } }
