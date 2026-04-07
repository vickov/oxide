//! Native function infrastructure.
//! NativeFn is a plain fn pointer so it can be stored in a Vec without Rc/Box.

use crate::heap::{JsHeap, HeapRef, value::{self, JsValue}};

/// A native Rust function callable from JS.
/// args[0] is always `this`; args[1..] are the call arguments.
/// Re-exported from heap to avoid circular imports.
pub use crate::heap::NativeFn;

/// Register a native function in the heap's natives table.
/// Returns a JsValue that can be stored as a property and called.
pub fn reg(heap: &mut JsHeap, func: NativeFn) -> JsValue {
    let id = heap.natives.len() as u32;
    heap.natives.push(func);
    value::from_native(id)
}

/// Set a named property on a heap object.
pub fn set(heap: &mut JsHeap, obj: HeapRef, name: &str, val: JsValue) {
    let nid = heap.strings.intern(name);
    if let Some(o) = heap.objects.get_mut(obj) {
        o.overflow.get_or_insert_with(Default::default).insert(nid, val);
    }
}

/// Set a named property to a native function on a heap object.
pub fn set_fn(heap: &mut JsHeap, obj: HeapRef, name: &str, func: NativeFn) {
    let v = reg(heap, func);
    set(heap, obj, name, v);
}

/// Get a named property from a heap object (overflow only, no prototype chain).
pub fn get_own(heap: &JsHeap, obj: HeapRef, name: &str) -> JsValue {
    match heap.strings.get_id(name) {
        Some(nid) => heap.objects.get(obj)
            .and_then(|o| o.overflow.as_ref())
            .and_then(|m| m.get(&nid))
            .copied()
            .unwrap_or(crate::heap::value::UNDEFINED),
        None => crate::heap::value::UNDEFINED,
    }
}

/// Read a numeric property from an object (array index or "length").
pub fn get_num_prop(heap: &JsHeap, obj: HeapRef, name: &str) -> f64 {
    use crate::vm::eval::js_to_number;
    let v = get_own(heap, obj, name);
    js_to_number(v, heap)
}

/// Read array length as usize.
pub fn array_len(heap: &JsHeap, obj: HeapRef) -> usize {
    let n = get_num_prop(heap, obj, "length");
    if n.is_nan() || n < 0.0 { 0 } else { n as usize }
}

/// Read element at index i from an array-like object.
pub fn array_get(heap: &JsHeap, obj: HeapRef, i: usize) -> JsValue {
    get_own(heap, obj, &i.to_string())
}

/// Write element at index i on an array-like object.
pub fn array_set(heap: &mut JsHeap, obj: HeapRef, i: usize, val: JsValue) {
    set(heap, obj, &i.to_string(), val);
}

/// Create a new empty array-like JsObject with length 0.
pub fn new_array(heap: &mut JsHeap) -> HeapRef {
    let r = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    let zero = value::from_int(0);
    set(heap, r, "length", zero);
    r
}

/// Get a property value from a heap object (mutable borrow for intern).
pub fn get_own_val(heap: &mut JsHeap, obj: HeapRef, name: &str) -> JsValue {
    let nid = heap.strings.intern(name);
    heap.objects.get(obj)
        .and_then(|o| o.overflow.as_ref())
        .and_then(|m| m.get(&nid))
        .copied()
        .unwrap_or(crate::heap::value::UNDEFINED)
}
