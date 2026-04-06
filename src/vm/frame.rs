//! Call frame — one activation record on the explicit call stack.

use crate::heap::value::JsValue;
use crate::heap::HeapRef;
use crate::heap::closure::BytecodeId;

pub struct CallFrame {
    pub bytecode_id: BytecodeId,
    pub ip:          usize,
    pub registers:   Vec<JsValue>,
    pub this_value:  JsValue,
    pub closure:     Option<HeapRef>,  // function object — for captured cells
}

impl CallFrame {
    pub fn new(bytecode_id: BytecodeId, reg_count: usize, this_value: JsValue) -> Self {
        Self {
            bytecode_id,
            ip: 0,
            registers: vec![crate::heap::value::UNDEFINED; reg_count],
            this_value,
            closure: None,
        }
    }

    pub fn reg(&self, r: u8) -> JsValue {
        self.registers.get(r as usize).copied().unwrap_or(crate::heap::value::UNDEFINED)
    }

    pub fn set_reg(&mut self, r: u8, v: JsValue) {
        if let Some(slot) = self.registers.get_mut(r as usize) {
            *slot = v;
        }
    }

    pub fn advance(&mut self) { self.ip += 1; }
}
