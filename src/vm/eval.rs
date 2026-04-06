//! Module 7 — Bytecode interpreter eval loop
//! Explicit Vec<CallFrame> call stack — stack overflow is JsException, never panic.
//! Full implementation: ~3 weeks, ~45 opcode cases.

use crate::compiler::Op;
use crate::heap::JsHeap;
use crate::heap::value::{self, JsValue, UNDEFINED};
use crate::runtime::promise::MicrotaskQueue;
use super::JsResult;
use super::exception::JsException;
use super::frame::CallFrame;

pub fn eval(
    _heap:       &mut JsHeap,
    frame:       &mut CallFrame,
    bytecode:    &[Op],
    _microtasks: &mut MicrotaskQueue,
) -> JsResult<JsValue> {
    loop {
        let op = bytecode.get(frame.ip)
            .ok_or_else(|| JsException::Internal("ip out of bounds".into()))?;

        match op {
            Op::Return { src } => return Ok(frame.reg(*src)),
            Op::Throw  { src } => return Err(JsException::Value(frame.reg(*src))),

            Op::LoadUndef { dst } => { frame.set_reg(*dst, UNDEFINED);           frame.advance(); }
            Op::LoadNull  { dst } => { frame.set_reg(*dst, value::NULL);          frame.advance(); }
            Op::LoadBool  { dst, val } => { frame.set_reg(*dst, value::from_bool(*val)); frame.advance(); }
            Op::LoadInt   { dst, val } => { frame.set_reg(*dst, value::from_int(*val));  frame.advance(); }

            Op::Jump { offset } => {
                frame.ip = (frame.ip as i64 + *offset as i64) as usize;
            }
            Op::JumpIfFalse { src, offset } => {
                let v = frame.reg(*src);
                if v == value::FALSE || v == UNDEFINED || v == value::NULL {
                    frame.ip = (frame.ip as i64 + *offset as i64) as usize;
                } else {
                    frame.advance();
                }
            }
            Op::JumpIfTrue { src, offset } => {
                let v = frame.reg(*src);
                if v == value::TRUE {
                    frame.ip = (frame.ip as i64 + *offset as i64) as usize;
                } else {
                    frame.advance();
                }
            }
            // Remaining ~38 opcodes: Module 7, ~3 weeks
            _ => return Err(JsException::Internal(
                format!("opcode not yet implemented: {:?}", op)
            )),
        }
    }
}
