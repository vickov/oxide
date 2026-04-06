//! Module 3 — Garbage Collector
//! Mark-and-sweep with generational collection and write barriers.
//! GC soundness is formally proven via Kani + Verus.
//!
//! # Safety
//! The GC traversal is the only place in Oxide that uses unsafe code.
//! All unsafe blocks must be justified and covered by Verus proofs.

use crate::heap::HeapRef;
use std::collections::HashSet;

/// GC state — mark bits, free list, generation tracking, remembered set.
pub struct GcState {
    pub mark_bits:      HashSet<u32>,
    pub old_gen:        HashSet<u32>,
    pub remembered_set: HashSet<u32>,   // old-gen objects pointing to young-gen
}

impl GcState {
    pub fn new() -> Self {
        Self {
            mark_bits:      HashSet::new(),
            old_gen:        HashSet::new(),
            remembered_set: HashSet::new(),
        }
    }

    pub fn mark(&mut self, r: HeapRef)         { self.mark_bits.insert(r.0); }
    pub fn is_marked(&self, r: HeapRef) -> bool { self.mark_bits.contains(&r.0) }
    pub fn clear_mark_bits(&mut self)          { self.mark_bits.clear(); }
    pub fn is_old_gen(&self, r: HeapRef) -> bool { self.old_gen.contains(&r.0) }
    pub fn is_young_gen(&self, r: HeapRef) -> bool { !self.old_gen.contains(&r.0) }
}

impl Default for GcState { fn default() -> Self { Self::new() } }

/// Write barrier — must run on every object field write.
/// Required for correct generational collection.
/// Nearly zero-cost on the fast path (young -> young writes).
#[inline(always)]
pub fn write_barrier(gc: &mut GcState, obj: HeapRef, new_value: crate::heap::value::JsValue) {
    // Fast path: if not in old generation, no barrier needed
    if !gc.is_old_gen(obj) { return; }

    // Slow path: old-gen object pointing to young-gen value
    if let Some(child_ref) = crate::heap::value::as_object(new_value) {
        if gc.is_young_gen(child_ref) {
            gc.remembered_set.insert(obj.0);
        }
    }
}
