//! Module 9 — Prototype Chain
//! Property lookup traverses the prototype chain on cache miss.
//! Built-in prototype chain registered at engine init.

use super::{JsHeap, HeapRef};
use super::value::{JsValue, StringId, UNDEFINED};

/// Walk the prototype chain looking for a property by name.
/// Returns UNDEFINED (not Nothing) if not found — matches JS semantics.
pub fn get_property(heap: &JsHeap, mut obj_ref: HeapRef, name: StringId) -> JsValue {
    loop {
        let obj = match heap.objects.get(obj_ref) {
            Some(o) => o,
            None    => return UNDEFINED,
        };

        // Check inline slots via shape
        if let Some(offset) = heap.shapes.slot_for(obj.shape, name) {
            return obj.slots[offset];
        }

        // Check overflow map
        if let Some(ref overflow) = obj.overflow {
            if let Some(&val) = overflow.get(&name) {
                return val;
            }
        }

        // Walk prototype chain
        match obj.prototype {
            Some(proto) => obj_ref = proto,
            None        => return UNDEFINED,
        }
    }
}
