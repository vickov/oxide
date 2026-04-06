//! Module 1 + 2 + 4 + 9 + 10 — Value, Heap, Object Model, Prototype, Closures
//!
//! The heap is the single most architecturally critical component.
//! All other modules depend on it being correct and stable.

pub mod value;
pub mod arena;
pub mod object;
pub mod prototype;
pub mod closure;

/// The JS heap — owns all JS objects, strings, functions, and cells.
/// HeapRef is always a u32 index — never a raw pointer.
pub struct JsHeap {
    pub objects:   arena::Arena<object::JsObject>,
    pub functions: arena::Arena<closure::JsFunction>,
    pub cells:     arena::Arena<closure::JsCell>,
    pub strings:   value::StringInterner,
    pub shapes:    object::ShapeTable,
    pub gc_state:  crate::gc::GcState,
}

impl JsHeap {
    pub fn new() -> Self {
        Self {
            objects:   arena::Arena::new(),
            functions: arena::Arena::new(),
            cells:     arena::Arena::new(),
            strings:   value::StringInterner::new(),
            shapes:    object::ShapeTable::new(),
            gc_state:  crate::gc::GcState::new(),
        }
    }
}

impl Default for JsHeap {
    fn default() -> Self { Self::new() }
}

/// Opaque reference into the heap — u32 index, never a raw pointer.
/// This eliminates Rust aliasing issues for cyclic JS object graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapRef(pub u32);
