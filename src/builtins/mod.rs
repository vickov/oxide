//! Module 8 -- Built-in objects (Priority 1+2)
//! Priority 1: Math, Number, console, JSON, Array, String, Object
//! Priority 2: Map, Set, WeakMap, WeakRef, Error types, Symbol (stubs)

pub mod native;
pub mod math;
pub mod number;
pub mod console;
pub mod json;
pub mod array;
pub mod string;

use crate::heap::{JsHeap, value};
use native::set;

/// Register all built-in objects on the global object.
/// Called once at engine init before any user code runs.
pub fn register_all(heap: &mut JsHeap) {
    let global = heap.global.get_or_insert_with(|| {
        heap.objects.alloc(crate::heap::object::JsObject::new(None))
    });
    let global = *global;

    // Math
    let math = math::create(heap);
    set(heap, global, "Math", value::from_object(math));

    // Number
    let number = number::create(heap);
    set(heap, global, "Number", value::from_object(number));
    // Global parseFloat / parseInt aliases
    let pf = native::get_own_val(heap, number, "parseFloat");
    let pi = native::get_own_val(heap, number, "parseInt");
    set(heap, global, "parseFloat", pf);
    set(heap, global, "parseInt",   pi);

    // console
    let con = console::create(heap);
    set(heap, global, "console", value::from_object(con));

    // JSON
    let json = json::create(heap);
    set(heap, global, "JSON", value::from_object(json));

    // Array (constructor object + prototype methods)
    let arr_ctor   = array::create_constructor(heap);
    let _arr_proto = array::create_prototype(heap);
    set(heap, global, "Array", value::from_object(arr_ctor));

    // String (constructor + prototype)
    let str_ctor  = string::create_constructor(heap);
    let _str_proto = string::create_prototype(heap);
    set(heap, global, "String", value::from_object(str_ctor));

    // Global constants
    set(heap, global, "undefined", value::UNDEFINED);
    set(heap, global, "NaN",       value::from_float(f64::NAN));
    set(heap, global, "Infinity",  value::from_float(f64::INFINITY));
    set(heap, global, "null",      value::NULL);
}
