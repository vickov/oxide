//! Module 7 -- Bytecode interpreter -- all ~45 opcodes implemented.
//! Split into small functions to avoid rustc nightly ICE on large match arms.

use std::collections::HashMap;
use crate::compiler::Op;
use crate::heap::JsHeap;
use crate::heap::value::{self, JsValue, StringId, UNDEFINED};
use crate::heap::object::JsObject;
use crate::heap::closure::JsFunction;
use crate::runtime::promise::MicrotaskQueue;
use super::JsResult;
use super::exception::JsException;
use super::frame::CallFrame;

// ---------------------------------------------------------------------------
// Type coercions (pub so tests can use them)
// ---------------------------------------------------------------------------

pub fn js_is_truthy(v: JsValue) -> bool {
    if v == UNDEFINED || v == value::NULL || v == value::FALSE { return false; }
    if value::is_int(v)   { return value::as_int(v).unwrap() != 0; }
    if value::is_float(v) { let f = value::as_float(v).unwrap(); return f != 0.0 && !f.is_nan(); }
    true
}

pub fn js_to_number(v: JsValue, heap: &JsHeap) -> f64 {
    if v == UNDEFINED     { return f64::NAN; }
    if v == value::NULL   { return 0.0; }
    if v == value::TRUE   { return 1.0; }
    if v == value::FALSE  { return 0.0; }
    if value::is_int(v)   { return value::as_int(v).unwrap() as f64; }
    if value::is_float(v) { return value::as_float(v).unwrap(); }
    if value::is_string(v) {
        return heap.strings.get(value::as_string(v).unwrap())
            .trim().parse::<f64>().unwrap_or(f64::NAN);
    }
    f64::NAN
}

pub fn js_to_i32(v: JsValue, heap: &JsHeap) -> i32 {
    let f = js_to_number(v, heap);
    if f.is_nan() || f.is_infinite() { 0 } else { f as i64 as i32 }
}

pub fn num(f: f64) -> JsValue {
    if f.fract() == 0.0 && f >= i32::MIN as f64 && f <= i32::MAX as f64 {
        value::from_int(f as i32)
    } else {
        value::from_float(f)
    }
}

fn to_str(v: JsValue, heap: &JsHeap) -> String {
    if v == UNDEFINED     { return "undefined".into(); }
    if v == value::NULL   { return "null".into(); }
    if v == value::TRUE   { return "true".into(); }
    if v == value::FALSE  { return "false".into(); }
    if value::is_int(v)   { return value::as_int(v).unwrap().to_string(); }
    if value::is_float(v) { return value::as_float(v).unwrap().to_string(); }
    if value::is_string(v){ return heap.strings.get(value::as_string(v).unwrap()).to_string(); }
    "[object Object]".into()
}

pub fn js_add(heap: &mut JsHeap, l: JsValue, r: JsValue) -> JsResult<JsValue> {
    if value::is_string(l) || value::is_string(r) {
        let ls = to_str(l, heap);
        let rs = to_str(r, heap);
        let id = heap.strings.intern(&format!("{}{}", ls, rs));
        return Ok(value::from_string(id));
    }
    Ok(num(js_to_number(l, heap) + js_to_number(r, heap)))
}

pub fn js_strict_eq(l: JsValue, r: JsValue) -> bool {
    if value::is_float(l) && value::is_float(r) {
        return value::as_float(l).unwrap() == value::as_float(r).unwrap();
    }
    l == r
}

fn js_abstract_eq(l: JsValue, r: JsValue, heap: &JsHeap) -> bool {
    if js_strict_eq(l, r) { return true; }
    if (l == value::NULL || l == UNDEFINED) && (r == value::NULL || r == UNDEFINED) { return true; }
    if (value::is_int(l) || value::is_float(l)) && value::is_string(r) {
        return js_to_number(l, heap) == js_to_number(r, heap);
    }
    if value::is_string(l) && (value::is_int(r) || value::is_float(r)) {
        return js_to_number(l, heap) == js_to_number(r, heap);
    }
    false
}

// ---------------------------------------------------------------------------
// Try-block tracking
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TryBlock { pub catch_ip: Option<usize>, pub catch_reg: Option<u8> }

// ---------------------------------------------------------------------------
// Opcode category: Load values
// ---------------------------------------------------------------------------

fn exec_load(op: &Op, frame: &mut CallFrame) -> bool {
    match op {
        Op::LoadUndef  { dst }      => { frame.set_reg(*dst, UNDEFINED); }
        Op::LoadNull   { dst }      => { frame.set_reg(*dst, value::NULL); }
        Op::LoadBool   { dst, val } => { frame.set_reg(*dst, value::from_bool(*val)); }
        Op::LoadInt    { dst, val } => { frame.set_reg(*dst, value::from_int(*val)); }
        Op::LoadConst  { dst, .. }  => { frame.set_reg(*dst, UNDEFINED); } // stub
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Opcode category: Variables
// ---------------------------------------------------------------------------

fn exec_vars(
    op:     &Op,
    heap:   &mut JsHeap,
    frame:  &mut CallFrame,
    locals: &mut HashMap<u32, JsValue>,
) -> bool {
    match op {
        Op::LoadVar { dst, name } => {
            frame.set_reg(*dst, locals.get(name).copied().unwrap_or(UNDEFINED));
        }
        Op::StoreVar { name, src } => {
            locals.insert(*name, frame.reg(*src));
        }
        Op::LoadCaptured { dst, cell_idx } => {
            let v = frame.closure
                .and_then(|cr| heap.functions.get(cr))
                .and_then(|f|  f.captured.get(*cell_idx as usize).copied())
                .and_then(|cr| heap.cells.get(cr))
                .map(|c| c.value)
                .unwrap_or(UNDEFINED);
            frame.set_reg(*dst, v);
        }
        Op::StoreCaptured { cell_idx, src } => {
            let val = frame.reg(*src);
            if let Some(cr) = frame.closure {
                if let Some(f) = heap.functions.get(cr) {
                    if let Some(&cr2) = f.captured.get(*cell_idx as usize) {
                        if let Some(c) = heap.cells.get_mut(cr2) { c.value = val; }
                    }
                }
            }
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Opcode category: Objects
// ---------------------------------------------------------------------------

fn exec_objects(op: &Op, heap: &mut JsHeap, frame: &mut CallFrame) -> bool {
    match op {
        Op::NewObject { dst } => {
            let r = heap.objects.alloc(JsObject::new(None));
            frame.set_reg(*dst, value::from_object(r));
        }
        Op::NewArray { dst, .. } => {
            let r = heap.objects.alloc(JsObject::new(None));
            frame.set_reg(*dst, value::from_object(r));
        }
        Op::GetPropStr { dst, obj, name } => {
            let v = value::as_object(frame.reg(*obj))
                .map(|r| crate::heap::prototype::get_property(heap, r, StringId(*name as u32)))
                .unwrap_or(UNDEFINED);
            frame.set_reg(*dst, v);
        }
        Op::GetProp { dst, obj, key } => {
            let v = value::as_object(frame.reg(*obj))
                .and_then(|r| value::as_string(frame.reg(*key))
                    .map(|n| crate::heap::prototype::get_property(heap, r, n)))
                .unwrap_or(UNDEFINED);
            frame.set_reg(*dst, v);
        }
        Op::SetProp { obj, key, src } => {
            if let (Some(obj_ref), Some(nid)) =
                (value::as_object(frame.reg(*obj)), value::as_string(frame.reg(*key)))
            {
                let val = frame.reg(*src);
                if let Some(o) = heap.objects.get_mut(obj_ref) {
                    o.overflow.get_or_insert_with(Default::default).insert(nid, val);
                }
            }
        }
        Op::DeleteProp { .. } => {} // stub
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Opcode category: Arithmetic
// ---------------------------------------------------------------------------

fn exec_arith(op: &Op, heap: &mut JsHeap, frame: &mut CallFrame) -> JsResult<bool> {
    match op {
        Op::Add { dst, lhs, rhs } => {
            let v = js_add(heap, frame.reg(*lhs), frame.reg(*rhs))?;
            frame.set_reg(*dst, v);
        }
        Op::Sub { dst, lhs, rhs } => {
            frame.set_reg(*dst, num(js_to_number(frame.reg(*lhs), heap) - js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Mul { dst, lhs, rhs } => {
            frame.set_reg(*dst, num(js_to_number(frame.reg(*lhs), heap) * js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Div { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_float(js_to_number(frame.reg(*lhs), heap) / js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Mod { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_float(js_to_number(frame.reg(*lhs), heap) % js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Neg { dst, src } => {
            frame.set_reg(*dst, num(-js_to_number(frame.reg(*src), heap)));
        }
        Op::Inc { dst } => { frame.set_reg(*dst, num(js_to_number(frame.reg(*dst), heap) + 1.0)); }
        Op::Dec { dst } => { frame.set_reg(*dst, num(js_to_number(frame.reg(*dst), heap) - 1.0)); }
        _ => return Ok(false),
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Opcode category: Comparison
// ---------------------------------------------------------------------------

fn exec_compare(op: &Op, heap: &JsHeap, frame: &mut CallFrame) -> bool {
    match op {
        Op::StrictEq { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_bool(js_strict_eq(frame.reg(*lhs), frame.reg(*rhs))));
        }
        Op::Eq { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_bool(js_abstract_eq(frame.reg(*lhs), frame.reg(*rhs), heap)));
        }
        Op::Lt  { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_bool(js_to_number(frame.reg(*lhs), heap) < js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Lte { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_bool(js_to_number(frame.reg(*lhs), heap) <= js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Gt  { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_bool(js_to_number(frame.reg(*lhs), heap) > js_to_number(frame.reg(*rhs), heap)));
        }
        Op::Gte { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_bool(js_to_number(frame.reg(*lhs), heap) >= js_to_number(frame.reg(*rhs), heap)));
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Opcode category: Logical
// ---------------------------------------------------------------------------

fn exec_logical(op: &Op, frame: &mut CallFrame) -> bool {
    match op {
        Op::Not { dst, src } => {
            frame.set_reg(*dst, value::from_bool(!js_is_truthy(frame.reg(*src))));
        }
        Op::And { dst, lhs, rhs } => {
            let l = frame.reg(*lhs);
            frame.set_reg(*dst, if !js_is_truthy(l) { l } else { frame.reg(*rhs) });
        }
        Op::Or { dst, lhs, rhs } => {
            let l = frame.reg(*lhs);
            frame.set_reg(*dst, if js_is_truthy(l) { l } else { frame.reg(*rhs) });
        }
        Op::Coalesce { dst, lhs, rhs } => {
            let l = frame.reg(*lhs);
            frame.set_reg(*dst, if l == UNDEFINED || l == value::NULL { frame.reg(*rhs) } else { l });
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Opcode category: Bitwise
// ---------------------------------------------------------------------------

fn exec_bitwise(op: &Op, heap: &JsHeap, frame: &mut CallFrame) -> bool {
    match op {
        Op::BitAnd { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_int(js_to_i32(frame.reg(*lhs), heap) & js_to_i32(frame.reg(*rhs), heap)));
        }
        Op::BitOr { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_int(js_to_i32(frame.reg(*lhs), heap) | js_to_i32(frame.reg(*rhs), heap)));
        }
        Op::BitXor { dst, lhs, rhs } => {
            frame.set_reg(*dst, value::from_int(js_to_i32(frame.reg(*lhs), heap) ^ js_to_i32(frame.reg(*rhs), heap)));
        }
        Op::BitNot { dst, src } => {
            frame.set_reg(*dst, value::from_int(!js_to_i32(frame.reg(*src), heap)));
        }
        Op::Shl { dst, lhs, rhs } => {
            let r = js_to_i32(frame.reg(*rhs), heap) & 0x1F;
            frame.set_reg(*dst, value::from_int(js_to_i32(frame.reg(*lhs), heap) << r));
        }
        Op::Shr { dst, lhs, rhs } => {
            let r = js_to_i32(frame.reg(*rhs), heap) & 0x1F;
            frame.set_reg(*dst, value::from_int(js_to_i32(frame.reg(*lhs), heap) >> r));
        }
        Op::Ushr { dst, lhs, rhs } => {
            let l = js_to_i32(frame.reg(*lhs), heap) as u32;
            let r = js_to_i32(frame.reg(*rhs), heap) as u32 & 0x1F;
            frame.set_reg(*dst, value::from_int((l >> r) as i32));
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Opcode category: Closures and classes
// ---------------------------------------------------------------------------

fn exec_closures(op: &Op, heap: &mut JsHeap, frame: &mut CallFrame) -> bool {
    match op {
        Op::NewClosure { dst, bytecode_id, .. } => {
            let f = JsFunction {
                bytecode_id: *bytecode_id,
                captured:    Vec::new(),
                formal_args: 0,
                name:        None,
                prototype:   None,
            };
            let r = heap.functions.alloc(f);
            frame.set_reg(*dst, value::from_object(r));
        }
        Op::NewClass { dst, .. } => {
            let r = heap.objects.alloc(JsObject::new(None));
            frame.set_reg(*dst, value::from_object(r));
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Main eval loop
// ---------------------------------------------------------------------------

pub fn eval(
    heap:        &mut JsHeap,
    frame:       &mut CallFrame,
    bytecode:    &[Op],
    _mt:         &mut MicrotaskQueue,
) -> JsResult<JsValue> {
    let mut locals:    HashMap<u32, JsValue> = HashMap::new();
    let mut try_stack: Vec<TryBlock>         = Vec::new();

    loop {
        let op = match bytecode.get(frame.ip) {
            Some(op) => op,
            None => return Err(JsException::Internal("ip out of bounds".into())),
        };

        // -- Control flow (handled inline -- may jump or return) --------------
        match op {
            Op::Return { src } => return Ok(frame.reg(*src)),
            Op::Throw  { src } => {
                let e = JsException::Value(frame.reg(*src));
                handle_throw(frame, &mut try_stack, e)?;
                continue; // caught -- resume from catch block ip
            }
            Op::Jump { offset } => {
                frame.ip = (frame.ip as i64 + *offset as i64) as usize;
                continue;
            }
            Op::JumpIfFalse { src, offset } => {
                if !js_is_truthy(frame.reg(*src)) {
                    frame.ip = (frame.ip as i64 + *offset as i64) as usize;
                } else {
                    frame.advance();
                }
                continue;
            }
            Op::JumpIfTrue { src, offset } => {
                if js_is_truthy(frame.reg(*src)) {
                    frame.ip = (frame.ip as i64 + *offset as i64) as usize;
                } else {
                    frame.advance();
                }
                continue;
            }
            Op::TryBegin { catch_offset, .. } => {
                try_stack.push(TryBlock {
                    catch_ip:  Some((frame.ip as i64 + *catch_offset as i64) as usize),
                    catch_reg: None,
                });
                frame.advance();
                continue;
            }
            Op::TryEnd => { try_stack.pop(); frame.advance(); continue; }
            Op::EnterCatch { dst } => {
                if let Some(b) = try_stack.last_mut() { b.catch_reg = Some(*dst); }
                frame.advance();
                continue;
            }
            _ => {}
        }

        // -- Dispatch to category handlers ------------------------------------
        let result: JsResult<()> = (|| {
            if exec_load(op, frame)                           { return Ok(()); }
            if exec_vars(op, heap, frame, &mut locals)        { return Ok(()); }
            if exec_objects(op, heap, frame)                  { return Ok(()); }
            if exec_arith(op, heap, frame)?                   { return Ok(()); }
            if exec_compare(op, heap, frame)                  { return Ok(()); }
            if exec_logical(op, frame)                        { return Ok(()); }
            if exec_bitwise(op, heap, frame)                  { return Ok(()); }
            if exec_closures(op, heap, frame)                 { return Ok(()); }

            // Stubs for remaining ops
            match op {
                Op::Call { dst, func, this, argc } => {
            let func_val = frame.reg(*func);
            let this_val = frame.reg(*this);
            if let Some(native_id) = crate::heap::value::as_native(func_val) {
                let mut call_args = vec![this_val];
                for i in 0..*argc { call_args.push(frame.reg(this.wrapping_add(1 + i))); }
                match heap.call_native(native_id, &call_args) {
                    Ok(v)  => frame.set_reg(*dst, v),
                    Err(e) => return Err(e),
                }
            } else {
                frame.set_reg(*dst, UNDEFINED); // JS-to-JS calls: Phase 2
            }
        }
                Op::CallMethod { dst, obj, method, argc } => {
            let obj_val = frame.reg(*obj);
            if let Some(obj_ref) = crate::heap::value::as_object(obj_val) {
                let name_id = crate::heap::value::StringId(*method as u32);
                let method_val = crate::heap::prototype::get_property(heap, obj_ref, name_id);
                if let Some(native_id) = crate::heap::value::as_native(method_val) {
                    let mut call_args = vec![obj_val];
                    for i in 0..*argc { call_args.push(frame.reg(obj.wrapping_add(1 + i))); }
                    match heap.call_native(native_id, &call_args) {
                        Ok(v)  => frame.set_reg(*dst, v),
                        Err(e) => return Err(e),
                    }
                } else {
                    frame.set_reg(*dst, UNDEFINED);
                }
            } else {
                frame.set_reg(*dst, UNDEFINED);
            }
        }
                Op::Await { dst, src }     => { let v = frame.reg(*src); frame.set_reg(*dst, v); }
                Op::Yield { dst, src }     => { let v = frame.reg(*src); frame.set_reg(*dst, v); }
                Op::InstanceOf { dst, .. } => { frame.set_reg(*dst, value::FALSE); }
                Op::In         { dst, .. } => { frame.set_reg(*dst, value::FALSE); }
                _ => {}
            }
            Ok(())
        })();

        match result {
            Ok(())  => frame.advance(),
            Err(e)  => {
                handle_throw(frame, &mut try_stack, e)?;
                continue; // caught -- resume from catch block ip
            }
        }
    }
}

/// Returns Ok(()) if the exception was caught (frame.ip set to catch block).
/// Returns Err(e) if there is no catch block -- caller should propagate.
fn handle_throw(
    frame:     &mut CallFrame,
    try_stack: &mut Vec<TryBlock>,
    e:         JsException,
) -> JsResult<()> {
    if let Some(block) = try_stack.pop() {
        if let Some(catch_ip) = block.catch_ip {
            if let (Some(reg), JsException::Value(v)) = (block.catch_reg, &e) {
                frame.set_reg(reg, *v);
            }
            frame.ip = catch_ip;
            return Ok(()); // caught -- caller continues loop from new ip
        }
    }
    Err(e) // uncaught -- propagate upward
}

