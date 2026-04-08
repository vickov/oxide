//! Call frame — one activation record on the explicit call stack.

use std::collections::HashMap;
use crate::heap::value::JsValue;
use crate::heap::HeapRef;
use crate::heap::closure::BytecodeId;
use super::eval::TryBlock;

pub struct CallFrame {
    pub bytecode_id: BytecodeId,
    pub ip:          usize,
    pub registers:   Vec<JsValue>,
    pub this_value:  JsValue,
    pub closure:     Option<HeapRef>,  // function object — for captured cells
    /// Register in the *caller* frame where the return value must be stored.
    /// None for the bottom frame (result returned directly from eval).
    pub ret_dst:     Option<u8>,
    /// Per-frame variable storage (LoadVar / StoreVar).
    pub locals:      HashMap<u32, JsValue>,
    /// Per-frame try/catch block stack.
    pub try_stack:   Vec<TryBlock>,
}

impl CallFrame {
    pub fn new(bytecode_id: BytecodeId, reg_count: usize, this_value: JsValue) -> Self {
        Self {
            bytecode_id,
            ip: 0,
            registers: vec![crate::heap::value::UNDEFINED; reg_count],
            this_value,
            closure: None,
            ret_dst: None,
            locals: HashMap::new(),
            try_stack: Vec::new(),
        }
    }

    pub fn reg(&self, r: u8) -> JsValue {
        self.registers.get(r as usize).copied().unwrap_or(crate::heap::value::UNDEFINED)
    }

    pub fn set_reg(&mut self, r: u8, v: JsValue) {
        if (r as usize) < self.registers.len() {
            self.registers[r as usize] = v;
        } else {
            // Grow register file on demand — callee may use more regs than pre-allocated
            self.registers.resize(r as usize + 1, crate::heap::value::UNDEFINED);
            self.registers[r as usize] = v;
        }
    }

    pub fn advance(&mut self) { self.ip += 1; }
}
