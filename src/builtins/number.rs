//! Number built-in object -- Priority 1
use crate::heap::{JsHeap, HeapRef, value::{self, JsValue, UNDEFINED}};
use crate::vm::exception::JsResult;
use crate::vm::eval::{js_to_number, num};
use super::native::{set_fn, set};

fn num_is_nan   (_h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let v = a.get(1).copied().unwrap_or(UNDEFINED);
    Ok(value::from_bool(value::is_float(v) && value::as_float(v).unwrap().is_nan()))
}
fn num_is_finite(_h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let v = a.get(1).copied().unwrap_or(UNDEFINED);
    if value::is_int(v) { return Ok(value::TRUE); }
    if value::is_float(v) { return Ok(value::from_bool(value::as_float(v).unwrap().is_finite())); }
    Ok(value::FALSE)
}
fn num_is_integer(_h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let v = a.get(1).copied().unwrap_or(UNDEFINED);
    if value::is_int(v) { return Ok(value::TRUE); }
    if value::is_float(v) {
        let f = value::as_float(v).unwrap();
        return Ok(value::from_bool(f.is_finite() && f.fract() == 0.0));
    }
    Ok(value::FALSE)
}
fn num_parse_float(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let v = a.get(1).copied().unwrap_or(UNDEFINED);
    if let Some(sid) = value::as_string(v) {
        let s = h.strings.get(sid).trim().to_string();
        return Ok(s.parse::<f64>().map(value::from_float).unwrap_or(value::from_float(f64::NAN)));
    }
    Ok(num(js_to_number(v, h)))
}
fn num_parse_int(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let v = a.get(1).copied().unwrap_or(UNDEFINED);
    let radix = a.get(2).copied().map(|r| js_to_number(r, h) as u32).unwrap_or(10);
    let radix = if radix == 0 { 10 } else { radix };
    if let Some(sid) = value::as_string(v) {
        let s = h.strings.get(sid).trim().to_string();
        if let Ok(n) = i64::from_str_radix(&s, radix) {
            return Ok(num(n as f64));
        }
        return Ok(value::from_float(f64::NAN));
    }
    Ok(num(js_to_number(v, h).trunc()))
}

pub fn create(heap: &mut JsHeap) -> HeapRef {
    let obj = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, obj, "isNaN",      num_is_nan);
    set_fn(heap, obj, "isFinite",   num_is_finite);
    set_fn(heap, obj, "isInteger",  num_is_integer);
    set_fn(heap, obj, "parseFloat", num_parse_float);
    set_fn(heap, obj, "parseInt",   num_parse_int);
    set(heap, obj, "MAX_SAFE_INTEGER", value::from_float(9007199254740991.0));
    set(heap, obj, "MIN_SAFE_INTEGER", value::from_float(-9007199254740991.0));
    set(heap, obj, "MAX_VALUE",        value::from_float(f64::MAX));
    set(heap, obj, "MIN_VALUE",        value::from_float(f64::MIN_POSITIVE));
    set(heap, obj, "NaN",              value::from_float(f64::NAN));
    set(heap, obj, "POSITIVE_INFINITY",value::from_float(f64::INFINITY));
    set(heap, obj, "NEGATIVE_INFINITY",value::from_float(f64::NEG_INFINITY));
    set(heap, obj, "EPSILON",          value::from_float(f64::EPSILON));
    obj
}
