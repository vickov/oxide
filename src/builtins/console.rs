//! console.* built-ins -- Priority 1
//! Output goes to stderr (serial in COBOS; replaceable via IoProvider callback).
use crate::heap::{JsHeap, HeapRef, value::JsValue};
use crate::vm::exception::JsResult;
use super::native::set_fn;

fn fmt_args(h: &mut JsHeap, args: &[JsValue]) -> String {
    args.iter().skip(1)
        .map(|&v| super::json::js_value_to_display(h, v, false))
        .collect::<Vec<_>>().join(" ")
}

fn con_log  (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { eprintln!("{}", fmt_args(h,a)); Ok(crate::heap::value::UNDEFINED) }
fn con_warn (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { eprintln!("[WARN] {}", fmt_args(h,a)); Ok(crate::heap::value::UNDEFINED) }
fn con_error(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { eprintln!("[ERR]  {}", fmt_args(h,a)); Ok(crate::heap::value::UNDEFINED) }
fn con_info (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { eprintln!("[INFO] {}", fmt_args(h,a)); Ok(crate::heap::value::UNDEFINED) }

pub fn create(heap: &mut JsHeap) -> HeapRef {
    let obj = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, obj, "log",   con_log);
    set_fn(heap, obj, "warn",  con_warn);
    set_fn(heap, obj, "error", con_error);
    set_fn(heap, obj, "info",  con_info);
    obj
}
