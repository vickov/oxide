//! Math built-in object -- Priority 1
use crate::heap::{JsHeap, HeapRef, value::{self, JsValue}};
use crate::vm::exception::JsResult;
use crate::vm::eval::{js_to_number, num};
use super::native::{set_fn, set};

fn arg0(heap: &mut JsHeap, args: &[JsValue]) -> f64 {
    args.get(1).copied().map(|v| js_to_number(v, heap)).unwrap_or(f64::NAN)
}
fn arg1(heap: &mut JsHeap, args: &[JsValue]) -> f64 {
    args.get(2).copied().map(|v| js_to_number(v, heap)).unwrap_or(f64::NAN)
}

fn math_floor (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(num(arg0(h,a).floor())) }
fn math_ceil  (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(num(arg0(h,a).ceil())) }
fn math_round (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(num(arg0(h,a).round())) }
fn math_trunc (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(num(arg0(h,a).trunc())) }
fn math_abs   (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(num(arg0(h,a).abs())) }
fn math_sqrt  (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).sqrt())) }
fn math_log   (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).ln())) }
fn math_log2  (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).log2())) }
fn math_log10 (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).log10())) }
fn math_sin   (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).sin())) }
fn math_cos   (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).cos())) }
fn math_tan   (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).tan())) }
fn math_sign  (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { Ok(value::from_float(arg0(h,a).signum())) }
fn math_random(_h: &mut JsHeap, _a: &[JsValue]) -> JsResult<JsValue> {
    // LCG pseudorandom -- good enough for Scope A
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(12345678901234567);
    let s = SEED.fetch_add(6364136223846793005, Ordering::Relaxed)
        .wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    SEED.store(s, Ordering::Relaxed);
    let f = (s >> 11) as f64 / (1u64 << 53) as f64;
    Ok(value::from_float(f))
}
fn math_pow(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    Ok(value::from_float(arg0(h,a).powf(arg1(h,a))))
}
fn math_min(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let mut m = f64::INFINITY;
    for &v in a.iter().skip(1) { let n = js_to_number(v, h); if n < m { m = n; } }
    Ok(num(m))
}
fn math_max(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let mut m = f64::NEG_INFINITY;
    for &v in a.iter().skip(1) { let n = js_to_number(v, h); if n > m { m = n; } }
    Ok(num(m))
}
fn math_hypot(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let sum: f64 = a.iter().skip(1).map(|&v| { let n = js_to_number(v, h); n * n }).sum();
    Ok(value::from_float(sum.sqrt()))
}

pub fn create(heap: &mut JsHeap) -> HeapRef {
    let obj = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, obj, "floor",  math_floor);
    set_fn(heap, obj, "ceil",   math_ceil);
    set_fn(heap, obj, "round",  math_round);
    set_fn(heap, obj, "trunc",  math_trunc);
    set_fn(heap, obj, "abs",    math_abs);
    set_fn(heap, obj, "sqrt",   math_sqrt);
    set_fn(heap, obj, "pow",    math_pow);
    set_fn(heap, obj, "log",    math_log);
    set_fn(heap, obj, "log2",   math_log2);
    set_fn(heap, obj, "log10",  math_log10);
    set_fn(heap, obj, "sin",    math_sin);
    set_fn(heap, obj, "cos",    math_cos);
    set_fn(heap, obj, "tan",    math_tan);
    set_fn(heap, obj, "min",    math_min);
    set_fn(heap, obj, "max",    math_max);
    set_fn(heap, obj, "random", math_random);
    set_fn(heap, obj, "sign",   math_sign);
    set_fn(heap, obj, "hypot",  math_hypot);
    set(heap, obj, "PI", value::from_float(std::f64::consts::PI));
    set(heap, obj, "E",  value::from_float(std::f64::consts::E));
    set(heap, obj, "LN2",   value::from_float(std::f64::consts::LN_2));
    set(heap, obj, "LN10",  value::from_float(std::f64::consts::LN_10));
    set(heap, obj, "SQRT2", value::from_float(std::f64::consts::SQRT_2));
    obj
}
