//! JSON.* built-ins -- Priority 1
use crate::heap::{JsHeap, HeapRef, value::{self, JsValue, UNDEFINED}};
use crate::vm::exception::{JsException, JsResult};
use super::native::{set_fn, set, array_len, array_get, new_array, array_set};

/// Convert a JsValue to a display string (for console.log).
pub fn js_value_to_display(heap: &mut JsHeap, v: JsValue, _json_mode: bool) -> String {
    if v == UNDEFINED      { return "undefined".into(); }
    if v == value::NULL    { return "null".into(); }
    if v == value::TRUE    { return "true".into(); }
    if v == value::FALSE   { return "false".into(); }
    if value::is_int(v)    { return value::as_int(v).unwrap().to_string(); }
    if value::is_float(v)  {
        let f = value::as_float(v).unwrap();
        if f.is_nan()      { return "NaN".into(); }
        if f.is_infinite() { return if f > 0.0 { "Infinity" } else { "-Infinity" }.into(); }
        return f.to_string();
    }
    if value::is_string(v) { return heap.strings.get(value::as_string(v).unwrap()).to_string(); }
    if value::is_native(v) { return "[Function]".into(); }
    if let Some(r) = value::as_object(v) {
        return stringify_object(heap, r, 0);
    }
    "undefined".into()
}

fn stringify_object(heap: &mut JsHeap, r: HeapRef, depth: usize) -> String {
    if depth > 10 { return "[Object]".into(); }
    // Check if array-like (has "length")
    let len_nid = heap.strings.intern("length");
    let has_len = heap.objects.get(r)
        .and_then(|o| o.overflow.as_ref())
        .map(|m| m.contains_key(&len_nid))
        .unwrap_or(false);

    if has_len {
        let len = array_len(heap, r);
        let mut parts = Vec::with_capacity(len);
        for i in 0..len {
            let v = array_get(heap, r, i);
            parts.push(stringify_value(heap, v, depth + 1));
        }
        return format!("[{}]", parts.join(","));
    }

    let entries: Vec<(String, JsValue)> = heap.objects.get(r)
        .and_then(|o| o.overflow.as_ref())
        .map(|m| m.iter()
            .map(|(&k, &v)| (heap.strings.get(k).to_string(), v))
            .collect())
        .unwrap_or_default();

    let parts: Vec<String> = entries.into_iter()
        .filter(|(_, v)| !value::is_native(*v) && *v != UNDEFINED)
        .map(|(k, v)| format!("\"{}\":{}", k, stringify_value(heap, v, depth + 1)))
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn stringify_value(heap: &mut JsHeap, v: JsValue, depth: usize) -> String {
    if v == UNDEFINED || value::is_native(v) { return "null".into(); }
    if v == value::NULL  { return "null".into(); }
    if v == value::TRUE  { return "true".into(); }
    if v == value::FALSE { return "false".into(); }
    if value::is_int(v)  { return value::as_int(v).unwrap().to_string(); }
    if value::is_float(v) {
        let f = value::as_float(v).unwrap();
        if f.is_nan() || f.is_infinite() { return "null".into(); }
        return f.to_string();
    }
    if value::is_string(v) {
        let s = heap.strings.get(value::as_string(v).unwrap());
        return format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    }
    if let Some(r) = value::as_object(v) {
        return stringify_object(heap, r, depth);
    }
    "null".into()
}

fn json_stringify(heap: &mut JsHeap, args: &[JsValue]) -> JsResult<JsValue> {
    let v = args.get(1).copied().unwrap_or(UNDEFINED);
    let s = stringify_value(heap, v, 0);
    let id = heap.strings.intern(&s);
    Ok(value::from_string(id))
}

fn json_parse(heap: &mut JsHeap, args: &[JsValue]) -> JsResult<JsValue> {
    let v = args.get(1).copied().unwrap_or(UNDEFINED);
    let s = if let Some(sid) = value::as_string(v) {
        heap.strings.get(sid).to_string()
    } else {
        return Err(JsException::type_error("JSON.parse requires a string"));
    };
    parse_json(heap, s.trim())
}

fn parse_json(heap: &mut JsHeap, s: &str) -> JsResult<JsValue> {
    let s = s.trim();
    if s == "null"  { return Ok(value::NULL); }
    if s == "true"  { return Ok(value::TRUE); }
    if s == "false" { return Ok(value::FALSE); }
    if s.starts_with('"') && s.ends_with('"') {
        let inner = &s[1..s.len()-1];
        let id = heap.strings.intern(&inner.replace("\\n","\n").replace("\\t","\t").replace("\\\"","\""));
        return Ok(value::from_string(id));
    }
    if let Ok(n) = s.parse::<f64>() { return Ok(crate::vm::eval::num(n)); }
    if s.starts_with('[') {
        let arr = new_array(heap);
        // Simple array parse -- handles flat arrays of primitives
        let inner = s[1..s.len()-1].trim();
        if inner.is_empty() { return Ok(value::from_object(arr)); }
        let mut i = 0usize;
        for (idx, item) in split_json_array(inner).iter().enumerate() {
            let v = parse_json(heap, item.trim())?;
            array_set(heap, arr, idx, v);
            i = idx + 1;
        }
        let len_v = value::from_int(i as i32);
        set(heap, arr, "length", len_v);
        return Ok(value::from_object(arr));
    }
    if s.starts_with('{') {
        let obj = heap.objects.alloc(crate::heap::object::JsObject::new(None));
        let inner = s[1..s.len()-1].trim();
        if !inner.is_empty() {
            for pair in split_json_array(inner) {
                if let Some(colon) = pair.find(':') {
                    let key_raw = pair[..colon].trim().trim_matches('"');
                    let val_raw = pair[colon+1..].trim();
                    let val = parse_json(heap, val_raw)?;
                    set(heap, obj, key_raw, val);
                }
            }
        }
        return Ok(value::from_object(obj));
    }
    Err(JsException::type_error(format!("JSON.parse: invalid JSON: {}", &s[..s.len().min(20)])))
}

fn split_json_array(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut start = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' if !in_str => in_str = true,
            '"' if in_str && (i == 0 || chars[i-1] != '\\') => in_str = false,
            '{' | '[' if !in_str => depth += 1,
            '}' | ']' if !in_str => depth -= 1,
            ',' if !in_str && depth == 0 => {
                parts.push(chars[start..i].iter().collect());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < chars.len() { parts.push(chars[start..].iter().collect()); }
    parts
}

pub fn create(heap: &mut JsHeap) -> HeapRef {
    let obj = heap.objects.alloc(crate::heap::object::JsObject::new(None));
    set_fn(heap, obj, "stringify", json_stringify);
    set_fn(heap, obj, "parse",     json_parse);
    obj
}
