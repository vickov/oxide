//! Module 10 -- Closures + Scope Chain
//! Mutable captured variables are heap-allocated cells.
//! Compiler resolves capture at compile time -- no runtime scope chain traversal.

use super::value::{JsValue, StringId, UNDEFINED};
use super::HeapRef;

pub type BytecodeId = u32;
pub type CellRef    = HeapRef;

/// A compiled JS function -- lives in heap.function_arena.
pub struct JsFunction {
    pub bytecode_id:  BytecodeId,
    pub captured:     Vec<CellRef>,
    pub formal_args:  u32,
    pub name:         Option<StringId>,
    pub prototype:    Option<HeapRef>,
}

/// A mutable closure capture cell -- lives in heap.cell_arena.
pub struct JsCell {
    pub value: JsValue,
}

impl JsCell {
    pub fn new(v: JsValue) -> Self { Self { value: v } }
    pub fn new_undefined() -> Self { Self { value: UNDEFINED } }
}
