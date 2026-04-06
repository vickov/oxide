//! Module 1 — Value Representation (NaN-boxing)
//!
//! Every JS value fits in a single 64-bit word.
//! No heap allocation for primitives.
//! Technique used by V8, JavaScriptCore, and QuickJS.

/// NaN-boxed 64-bit JS value.
/// IEEE 754 NaN space encodes all non-float types.
/// 8 bytes per value, no allocation for undefined/null/bool/int/float.
pub type JsValue = u64;

/// String identity — index into the StringInterner.
/// Equal strings share one allocation. Comparison is integer equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(pub u32);

/// Symbol identity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

// NaN-box tag constants
// IEEE 754: any value with exponent = all-ones and mantissa != 0 is NaN.
// We use the quiet NaN bit (bit 51) to signal a tagged value.
const NAN_MASK:       u64 = 0xFFF8_0000_0000_0000;
const TAG_UNDEFINED:  u64 = NAN_MASK | 0x0000_0000_0000_0001;
const TAG_NULL:       u64 = NAN_MASK | 0x0000_0000_0000_0002;
const TAG_FALSE:      u64 = NAN_MASK | 0x0000_0000_0000_0003;
const TAG_TRUE:       u64 = NAN_MASK | 0x0000_0000_0000_0004;
const TAG_INT:        u64 = NAN_MASK | 0x0001_0000_0000_0000;
const TAG_STRING:     u64 = NAN_MASK | 0x0002_0000_0000_0000;
const TAG_OBJECT:     u64 = NAN_MASK | 0x0003_0000_0000_0000;
const TAG_SYMBOL:     u64 = NAN_MASK | 0x0004_0000_0000_0000;

pub const UNDEFINED: JsValue = TAG_UNDEFINED;
pub const NULL:      JsValue = TAG_NULL;
pub const FALSE:     JsValue = TAG_FALSE;
pub const TRUE:      JsValue = TAG_TRUE;

pub fn from_bool(b: bool) -> JsValue { if b { TRUE } else { FALSE } }
pub fn from_int(n: i32)   -> JsValue { TAG_INT | (n as u32 as u64) }
pub fn from_float(f: f64) -> JsValue { f.to_bits() }
pub fn from_string(id: StringId)  -> JsValue { TAG_STRING | id.0 as u64 }
pub fn from_object(r: super::HeapRef) -> JsValue { TAG_OBJECT | r.0 as u64 }

pub fn is_undefined(v: JsValue) -> bool { v == UNDEFINED }
pub fn is_null(v: JsValue)      -> bool { v == NULL }
pub fn is_bool(v: JsValue)      -> bool { v == TRUE || v == FALSE }
pub fn is_int(v: JsValue)       -> bool { v & 0xFFFF_0000_0000_0000 == TAG_INT }
pub fn is_float(v: JsValue)     -> bool { v & NAN_MASK != NAN_MASK }
pub fn is_string(v: JsValue)    -> bool { v & 0xFFFF_0000_0000_0000 == TAG_STRING }
pub fn is_object(v: JsValue)    -> bool { v & 0xFFFF_0000_0000_0000 == TAG_OBJECT }

pub fn as_bool(v: JsValue)   -> Option<bool>            { if is_bool(v) { Some(v == TRUE) } else { None } }
pub fn as_int(v: JsValue)    -> Option<i32>             { if is_int(v) { Some((v & 0xFFFF_FFFF) as i32) } else { None } }
pub fn as_float(v: JsValue)  -> Option<f64>             { if is_float(v) { Some(f64::from_bits(v)) } else { None } }
pub fn as_string(v: JsValue) -> Option<StringId>        { if is_string(v) { Some(StringId((v & 0xFFFF_FFFF) as u32)) } else { None } }
pub fn as_object(v: JsValue) -> Option<super::HeapRef>  { if is_object(v) { Some(super::HeapRef((v & 0xFFFF_FFFF) as u32)) } else { None } }

/// Content-addressed string interning.
/// Equal strings share one allocation. Comparison is O(1).
pub struct StringInterner {
    strings: Vec<String>,
    index:   std::collections::HashMap<String, StringId>,
}

impl StringInterner {
    pub fn new() -> Self { Self { strings: Vec::new(), index: std::collections::HashMap::new() } }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.index.get(s) { return id; }
        let id = StringId(self.strings.len() as u32);
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), id);
        id
    }

    pub fn get(&self, id: StringId) -> &str { &self.strings[id.0 as usize] }
}

impl Default for StringInterner { fn default() -> Self { Self::new() } }
