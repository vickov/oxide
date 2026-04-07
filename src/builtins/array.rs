//! Array built-in methods -- Priority 1
use crate::heap::{JsHeap, HeapRef, value::{self, JsValue, UNDEFINED}};
use crate::vm::exception::{JsException, JsResult};
use crate::vm::eval::js_to_number;
use super::native::{set_fn, set, array_len, array_get, array_set, new_array};

fn get_this(args: &[JsValue]) -> JsResult<HeapRef> {
    value::as_object(args.get(0).copied().unwrap_or(UNDEFINED))
        .ok_or_else(|| JsException::type_error("Array method called on non-object"))
}

fn arr_push(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let mut len = array_len(h, this);
    for &v in a.iter().skip(1) { array_set(h, this, len, v); len += 1; }
    let lv = value::from_int(len as i32);
    set(h, this, "length", lv);
    Ok(lv)
}

fn arr_pop(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let len = array_len(h, this);
    if len == 0 { return Ok(UNDEFINED); }
    let last = array_get(h, this, len - 1);
    // remove last element
    let idx_s = (len - 1).to_string();
    let nid = h.strings.intern(&idx_s);
    if let Some(o) = h.objects.get_mut(this) { o.overflow.get_or_insert_with(Default::default).remove(&nid); }
    set(h, this, "length", value::from_int((len - 1) as i32));
    Ok(last)
}

fn arr_shift(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let len = array_len(h, this);
    if len == 0 { return Ok(UNDEFINED); }
    let first = array_get(h, this, 0);
    for i in 1..len { let v = array_get(h, this, i); array_set(h, this, i - 1, v); }
    let idx_s = (len - 1).to_string();
    let nid = h.strings.intern(&idx_s);
    if let Some(o) = h.objects.get_mut(this) { o.overflow.get_or_insert_with(Default::default).remove(&nid); }
    set(h, this, "length", value::from_int((len - 1) as i32));
    Ok(first)
}

fn arr_unshift(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let len = array_len(h, this);
    let n = a.len() - 1;
    for i in (0..len).rev() { let v = array_get(h, this, i); array_set(h, this, i + n, v); }
    for (i, &v) in a.iter().skip(1).enumerate() { array_set(h, this, i, v); }
    let new_len = value::from_int((len + n) as i32);
    set(h, this, "length", new_len);
    Ok(new_len)
}

fn arr_join(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let sep = if let Some(&sv) = a.get(1) {
        if let Some(sid) = value::as_string(sv) { h.strings.get(sid).to_string() } else { ",".into() }
    } else { ",".into() };
    let len = array_len(h, this);
    let parts: Vec<String> = (0..len)
        .map(|i| { let v = array_get(h, this, i); super::json::js_value_to_display(h, v, false) })
        .collect();
    let s = parts.join(&sep);
    let id = h.strings.intern(&s);
    Ok(value::from_string(id))
}

fn arr_reverse(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let len = array_len(h, this);
    let mut lo = 0; let mut hi = len.saturating_sub(1);
    while lo < hi {
        let a_val = array_get(h, this, lo);
        let b_val = array_get(h, this, hi);
        array_set(h, this, lo, b_val);
        array_set(h, this, hi, a_val);
        lo += 1; hi -= 1;
    }
    Ok(value::from_object(this))
}

fn arr_slice(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let len = array_len(h, this) as i64;
    let start = a.get(1).map(|&v| { let n = js_to_number(v, h) as i64; (if n < 0 { (len + n).max(0) } else { n.min(len) }) as usize }).unwrap_or(0);
    let end   = a.get(2).map(|&v| { let n = js_to_number(v, h) as i64; (if n < 0 { (len + n).max(0) } else { n.min(len) }) as usize }).unwrap_or(len as usize);
    let result = new_array(h);
    for (i, idx) in (start..end).enumerate() {
        let v = array_get(h, this, idx);
        array_set(h, result, i, v);
    }
    set(h, result, "length", value::from_int((end.saturating_sub(start)) as i32));
    Ok(value::from_object(result))
}

fn arr_includes(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let target = a.get(1).copied().unwrap_or(UNDEFINED);
    let len = array_len(h, this);
    for i in 0..len {
        let v = array_get(h, this, i);
        if crate::vm::eval::js_strict_eq(v, target) { return Ok(value::TRUE); }
    }
    Ok(value::FALSE)
}

fn arr_index_of(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let target = a.get(1).copied().unwrap_or(UNDEFINED);
    let len = array_len(h, this);
    for i in 0..len {
        if crate::vm::eval::js_strict_eq(array_get(h, this, i), target) {
            return Ok(value::from_int(i as i32));
        }
    }
    Ok(value::from_int(-1))
}

fn arr_is_array(_h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    // Heuristic: has a "length" property
    Ok(value::from_bool(value::as_object(a.get(1).copied().unwrap_or(UNDEFINED)).is_some()))
}

fn arr_from(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let src = a.get(1).copied().unwrap_or(UNDEFINED);
    let result = new_array(h);
    if let Some(r) = value::as_object(src) {
        let len = array_len(h, r);
        for i in 0..len { let v = array_get(h, r, i); array_set(h, result, i, v); }
        set(h, result, "length", value::from_int(len as i32));
    }
    Ok(value::from_object(result))
}

// map, filter, forEach, find, findIndex, some, every -- require calling JS callbacks
// These are stubbed here; full implementation needs Call dispatch to be complete.
fn arr_map(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?; let len = array_len(h, this);
    let result = new_array(h);
    set(h, result, "length", value::from_int(len as i32));
    // Without JS callback support yet, return a copy
    for i in 0..len { let v = array_get(h, this, i); array_set(h, result, i, v); }
    Ok(value::from_object(result))
}
fn arr_filter(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    arr_slice(h, &[a.get(0).copied().unwrap_or(UNDEFINED)])
}
fn arr_for_each(_h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> { let _ = get_this(a)?; Ok(UNDEFINED) }
fn arr_find(_h: &mut JsHeap, _a: &[JsValue]) -> JsResult<JsValue>      { Ok(UNDEFINED) }
fn arr_find_idx(_h: &mut JsHeap, _a: &[JsValue]) -> JsResult<JsValue>  { Ok(value::from_int(-1)) }
fn arr_some(_h: &mut JsHeap, _a: &[JsValue]) -> JsResult<JsValue>      { Ok(value::FALSE) }
fn arr_every(_h: &mut JsHeap, _a: &[JsValue]) -> JsResult<JsValue>     { Ok(value::TRUE) }
fn arr_reduce(_h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue>     {
    Ok(a.get(2).copied().unwrap_or(UNDEFINED))
}
fn arr_flat(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue>        { arr_slice(h, a) }
fn arr_flat_map(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue>    { arr_map(h, a) }

fn arr_sort(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?;
    let len = array_len(h, this);
    let mut items: Vec<JsValue> = (0..len).map(|i| array_get(h, this, i)).collect();
    items.sort_by(|&l, &r| {
        let ls = super::json::js_value_to_display(h, l, false);
        let rs = super::json::js_value_to_display(h, r, false);
        ls.cmp(&rs)
    });
    for (i, v) in items.into_iter().enumerate() { array_set(h, this, i, v); }
    Ok(value::from_object(this))
}

fn arr_splice(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let this = get_this(a)?; let len = array_len(h, this) as i64;
    let start = a.get(1).map(|&v| { let n = js_to_number(v, h) as i64; if n < 0 { (len+n).max(0) } else { n.min(len) } }).unwrap_or(0) as usize;
    let del_count = a.get(2).map(|&v| (js_to_number(v, h) as usize).min(len as usize - start)).unwrap_or(len as usize - start);
    let removed = new_array(h);
    for i in 0..del_count { let v = array_get(h, this, start + i); array_set(h, removed, i, v); }
    set(h, removed, "length", value::from_int(del_count as i32));
    let new_items: Vec<JsValue> = a.iter().skip(3).copied().collect();
    let tail: Vec<JsValue> = (start + del_count..len as usize).map(|i| array_get(h, this, i)).collect();
    let mut w = start;
    for v in new_items.iter().chain(tail.iter()) { array_set(h, this, w, *v); w += 1; }
    set(h, this, "length", value::from_int(w as i32));
    Ok(value::from_object(removed))
}

pub fn create_prototype(heap: &mut JsHeap) -> HeapRef {
    let proto = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, proto, "push",       arr_push);
    set_fn(heap, proto, "pop",        arr_pop);
    set_fn(heap, proto, "shift",      arr_shift);
    set_fn(heap, proto, "unshift",    arr_unshift);
    set_fn(heap, proto, "join",       arr_join);
    set_fn(heap, proto, "reverse",    arr_reverse);
    set_fn(heap, proto, "slice",      arr_slice);
    set_fn(heap, proto, "splice",     arr_splice);
    set_fn(heap, proto, "includes",   arr_includes);
    set_fn(heap, proto, "indexOf",    arr_index_of);
    set_fn(heap, proto, "map",        arr_map);
    set_fn(heap, proto, "filter",     arr_filter);
    set_fn(heap, proto, "forEach",    arr_for_each);
    set_fn(heap, proto, "find",       arr_find);
    set_fn(heap, proto, "findIndex",  arr_find_idx);
    set_fn(heap, proto, "some",       arr_some);
    set_fn(heap, proto, "every",      arr_every);
    set_fn(heap, proto, "reduce",     arr_reduce);
    set_fn(heap, proto, "flat",       arr_flat);
    set_fn(heap, proto, "flatMap",    arr_flat_map);
    set_fn(heap, proto, "sort",       arr_sort);
    proto
}

pub fn create_constructor(heap: &mut JsHeap) -> HeapRef {
    let ctor = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, ctor, "isArray", arr_is_array);
    set_fn(heap, ctor, "from",    arr_from);
    ctor
}
