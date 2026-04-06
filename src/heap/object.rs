//! Module 4 — Object Model + Hidden Classes (Shapes)
//! JS objects are dictionaries. The shape system makes them struct-like for JIT.

use super::value::{JsValue, StringId};
use super::HeapRef;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub u32);

/// A JS object — shape-indexed inline slots with HashMap overflow.
pub struct JsObject {
    pub shape:     ShapeId,
    pub slots:     [JsValue; 8],           // inline fast-path slots
    pub overflow:  Option<HashMap<StringId, JsValue>>,
    pub prototype: Option<HeapRef>,
}

impl JsObject {
    pub fn new(prototype: Option<HeapRef>) -> Self {
        Self {
            shape:     ShapeId(0),
            slots:     [super::value::UNDEFINED; 8],
            overflow:  None,
            prototype,
        }
    }

    /// Iterate all HeapRef values reachable from this object (for GC mark phase).
    pub fn references(&self) -> impl Iterator<Item = HeapRef> + '_ {
        let slot_refs = self.slots.iter().filter_map(|&v| super::value::as_object(v));
        let overflow_refs = self.overflow.iter()
            .flat_map(|m| m.values())
            .filter_map(|&v| super::value::as_object(v));
        let proto_ref = self.prototype.into_iter();
        slot_refs.chain(overflow_refs).chain(proto_ref)
    }
}

/// Hidden class — describes property layout for a set of objects.
/// Objects with the same shape share the layout descriptor.
pub struct Shape {
    pub property_names:   Vec<StringId>,
    pub property_offsets: Vec<u32>,
    pub parent:           Option<ShapeId>,
    pub transitions:      HashMap<StringId, ShapeId>,
}

/// Global shape registry — shared across the heap.
pub struct ShapeTable {
    shapes: Vec<Shape>,
}

impl ShapeTable {
    pub fn new() -> Self {
        // Shape 0 = empty object shape
        Self { shapes: vec![Shape {
            property_names:   Vec::new(),
            property_offsets: Vec::new(),
            parent:           None,
            transitions:      HashMap::new(),
        }] }
    }

    pub fn get(&self, id: ShapeId) -> &Shape { &self.shapes[id.0 as usize] }

    pub fn slot_for(&self, id: ShapeId, name: StringId) -> Option<usize> {
        let shape = self.get(id);
        shape.property_names.iter().position(|&n| n == name)
    }
}

impl Default for ShapeTable { fn default() -> Self { Self::new() } }
