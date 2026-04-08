//! Module 12 — Exception handling
//! Stack overflow => JsException::StackOverflow, never a Rust panic.

use crate::heap::value::JsValue;

pub type JsResult<T> = Result<T, JsException>;

#[derive(Debug, Clone)]
pub enum JsException {
    /// throw expr — any JS value can be thrown
    Value(JsValue),
    /// Call stack depth exceeded the hard limit
    StackOverflow,
    /// Execution fuel exhausted — too many bytecode instructions
    FuelExhausted,
    /// Engine bug — should never reach production
    Internal(String),
}

impl JsException {
    pub fn type_error(msg: impl Into<String>) -> Self {
        Self::Internal(format!("TypeError: {}", msg.into()))
    }
    pub fn range_error(msg: impl Into<String>) -> Self {
        Self::Internal(format!("RangeError: {}", msg.into()))
    }
    pub fn reference_error(msg: impl Into<String>) -> Self {
        Self::Internal(format!("ReferenceError: {}", msg.into()))
    }
}

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub function_name: Option<crate::heap::value::StringId>,
    pub bytecode_id:   u32,
    pub ip:            usize,
}

pub fn capture_stack_trace(stack: &super::CallStack) -> Vec<FrameInfo> {
    stack.frames.iter().rev().map(|f| FrameInfo {
        function_name: None,
        bytecode_id:   f.bytecode_id,
        ip:            f.ip,
    }).collect()
}
