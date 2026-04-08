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
/// A native Rust function callable from JS. args[0] = this, args[1..] = call args.
pub type NativeFn = fn(&mut JsHeap, &[value::JsValue]) -> crate::vm::exception::JsResult<value::JsValue>;

pub struct JsHeap {
    pub objects:   arena::Arena<object::JsObject>,
    pub functions: arena::Arena<closure::JsFunction>,
    pub cells:     arena::Arena<closure::JsCell>,
    pub strings:   value::StringInterner,
    pub shapes:    object::ShapeTable,
    pub gc_state:  crate::gc::GcState,
    pub natives:   Vec<NativeFn>,
    pub global:    Option<HeapRef>,
    pub bytecodes: Vec<crate::compiler::Bytecode>,
    /// Pending microtasks enqueued by Promise resolve/reject.
    /// Each entry is (reaction_heap_ref, settled_value).
    /// Drained by MicrotaskQueue::drain() and JsEngine::drain_microtasks().
    pub pending_microtasks: std::collections::VecDeque<(HeapRef, value::JsValue)>,
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
            natives:   Vec::new(),
            global:    None,
            bytecodes: Vec::new(),
            pending_microtasks: std::collections::VecDeque::new(),
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

impl JsHeap {
    pub fn call_native(&mut self, id: u32, args: &[crate::heap::value::JsValue])
        -> crate::vm::exception::JsResult<crate::heap::value::JsValue>
    {
        let func = self.natives.get(id as usize).copied()
            .ok_or_else(|| crate::vm::exception::JsException::Internal(
                format!("native fn {} not registered", id)))?;
        func(self, args)
    }
}
