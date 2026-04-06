//! String built-in methods -- Priority 1
use crate::heap::{JsHeap, HeapRef, value::{self, JsValue, UNDEFINED}};
use crate::vm::exception::{JsException, JsResult};
use crate::vm::eval::js_to_number;
use super::native::{set_fn, set, new_array, array_set};

fn get_str(h: &mut JsHeap, v: JsValue) -> JsResult<String> {
    if let Some(sid) = value::as_string(v) { return Ok(h.strings.get(sid).to_string()); }
    Err(JsException::type_error("String method on non-string"))
}
fn this_str(h: &mut JsHeap, a: &[JsValue]) -> JsResult<String> {
    get_str(h, a.get(0).copied().unwrap_or(UNDEFINED))
}
fn ret_str(h: &mut JsHeap, s: String) -> JsValue { value::from_string(h.strings.intern(&s)) }

fn str_len (h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?; Ok(value::from_int(s.len() as i32))
}
fn str_to_lower(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?.to_lowercase(); Ok(ret_str(h, s))
}
fn str_to_upper(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?.to_uppercase(); Ok(ret_str(h, s))
}
fn str_trim(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?.trim().to_string(); Ok(ret_str(h, s))
}
fn str_trim_start(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?.trim_start().to_string(); Ok(ret_str(h, s))
}
fn str_trim_end(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?.trim_end().to_string(); Ok(ret_str(h, s))
}
fn str_slice(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?; let len = s.len() as i64;
    let start = a.get(1).map(|&v| { let n = js_to_number(v, h) as i64; if n < 0 { (len+n).max(0) } else { n.min(len) } }).unwrap_or(0) as usize;
    let end   = a.get(2).map(|&v| { let n = js_to_number(v, h) as i64; if n < 0 { (len+n).max(0) } else { n.min(len) } }).unwrap_or(len) as usize;
    let result = if start >= end { "".to_string() } else { s[start..end.min(s.len())].to_string() };
    Ok(ret_str(h, result))
}
fn str_substring(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?; let len = s.len();
    let mut start = a.get(1).map(|&v| js_to_number(v, h) as usize).unwrap_or(0).min(len);
    let mut end   = a.get(2).map(|&v| js_to_number(v, h) as usize).unwrap_or(len).min(len);
    if start > end { std::mem::swap(&mut start, &mut end); }
    Ok(ret_str(h, s[start..end].to_string()))
}
fn str_index_of(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let needle = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(value::from_int(s.find(&*needle).map(|i| i as i32).unwrap_or(-1)))
}
fn str_last_index_of(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let needle = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(value::from_int(s.rfind(&*needle).map(|i| i as i32).unwrap_or(-1)))
}
fn str_includes(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let needle = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(value::from_bool(s.contains(&*needle)))
}
fn str_starts_with(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let needle = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(value::from_bool(s.starts_with(&*needle)))
}
fn str_ends_with(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let needle = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(value::from_bool(s.ends_with(&*needle)))
}
fn str_replace(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let from = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    let to   = get_str(h, a.get(2).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(ret_str(h, s.replacen(&*from, &to, 1)))
}
fn str_replace_all(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let from = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    let to   = get_str(h, a.get(2).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    Ok(ret_str(h, s.replace(&*from, &to)))
}
fn str_pad_start(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let target = js_to_number(a.get(1).copied().unwrap_or(UNDEFINED), h) as usize;
    let pad = get_str(h, a.get(2).copied().unwrap_or(UNDEFINED)).unwrap_or(" ".into());
    if s.len() >= target { return Ok(ret_str(h, s)); }
    let need = target - s.len();
    let padding: String = pad.chars().cycle().take(need).collect();
    Ok(ret_str(h, format!("{}{}", padding, s)))
}
fn str_pad_end(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let target = js_to_number(a.get(1).copied().unwrap_or(UNDEFINED), h) as usize;
    let pad = get_str(h, a.get(2).copied().unwrap_or(UNDEFINED)).unwrap_or(" ".into());
    if s.len() >= target { return Ok(ret_str(h, s)); }
    let need = target - s.len();
    let padding: String = pad.chars().cycle().take(need).collect();
    Ok(ret_str(h, format!("{}{}", s, padding)))
}
fn str_split(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let sep = get_str(h, a.get(1).copied().unwrap_or(UNDEFINED)).unwrap_or_default();
    let parts: Vec<&str> = if sep.is_empty() {
        s.char_indices().map(|(i, _)| &s[i..i+1]).collect()
    } else {
        s.split(&*sep).collect()
    };
    let arr = new_array(h);
    for (i, p) in parts.iter().enumerate() {
        let id = h.strings.intern(p);
        array_set(h, arr, i, value::from_string(id));
    }
    set(h, arr, "length", value::from_int(parts.len() as i32));
    Ok(value::from_object(arr))
}
fn str_char_at(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let idx = js_to_number(a.get(1).copied().unwrap_or(UNDEFINED), h) as usize;
    let c = s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default();
    Ok(ret_str(h, c))
}
fn str_char_code_at(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let idx = js_to_number(a.get(1).copied().unwrap_or(UNDEFINED), h) as usize;
    let code = s.chars().nth(idx).map(|c| c as u32 as i32).unwrap_or(-1);
    Ok(value::from_int(code))
}
fn str_repeat(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s = this_str(h, a)?;
    let n = js_to_number(a.get(1).copied().unwrap_or(UNDEFINED), h) as usize;
    Ok(ret_str(h, s.repeat(n)))
}
fn str_concat(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let mut s = this_str(h, a)?;
    for &v in a.iter().skip(1) {
        if let Some(sid) = value::as_string(v) { s.push_str(h.strings.get(sid)); }
    }
    Ok(ret_str(h, s))
}
fn str_from_char_code(h: &mut JsHeap, a: &[JsValue]) -> JsResult<JsValue> {
    let s: String = a.iter().skip(1)
        .filter_map(|&v| { let n = js_to_number(v, h) as u32; char::from_u32(n) })
        .collect();
    Ok(ret_str(h, s))
}

pub fn create_prototype(heap: &mut JsHeap) -> HeapRef {
    let proto = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, proto, "length",       str_len);
    set_fn(heap, proto, "toLowerCase",  str_to_lower);
    set_fn(heap, proto, "toUpperCase",  str_to_upper);
    set_fn(heap, proto, "trim",         str_trim);
    set_fn(heap, proto, "trimStart",    str_trim_start);
    set_fn(heap, proto, "trimEnd",      str_trim_end);
    set_fn(heap, proto, "slice",        str_slice);
    set_fn(heap, proto, "substring",    str_substring);
    set_fn(heap, proto, "indexOf",      str_index_of);
    set_fn(heap, proto, "lastIndexOf",  str_last_index_of);
    set_fn(heap, proto, "includes",     str_includes);
    set_fn(heap, proto, "startsWith",   str_starts_with);
    set_fn(heap, proto, "endsWith",     str_ends_with);
    set_fn(heap, proto, "replace",      str_replace);
    set_fn(heap, proto, "replaceAll",   str_replace_all);
    set_fn(heap, proto, "padStart",     str_pad_start);
    set_fn(heap, proto, "padEnd",       str_pad_end);
    set_fn(heap, proto, "split",        str_split);
    set_fn(heap, proto, "charAt",       str_char_at);
    set_fn(heap, proto, "charCodeAt",   str_char_code_at);
    set_fn(heap, proto, "repeat",       str_repeat);
    set_fn(heap, proto, "concat",       str_concat);
    proto
}

pub fn create_constructor(heap: &mut JsHeap) -> HeapRef {
    let ctor = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, ctor, "fromCharCode", str_from_char_code);
    ctor
}
