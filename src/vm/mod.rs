//! Module 7 — Bytecode Interpreter
//! Executes bytecode instructions one at a time.
//! Call stack is an explicit Vec<CallFrame> — not the Rust call stack.
//! Stack overflow => JsException, never a Rust panic.

pub mod frame;
pub mod eval;
pub mod exception;

pub use exception::{JsException, JsResult};

const MAX_CALL_DEPTH: usize = 10_000;

/// The call stack — explicit, bounded, never uses the Rust stack.
pub struct CallStack {
    pub frames:    Vec<frame::CallFrame>,
    pub max_depth: usize,
}

impl CallStack {
    pub fn new() -> Self {
        Self { frames: Vec::new(), max_depth: MAX_CALL_DEPTH }
    }

    pub fn push(&mut self, frame: frame::CallFrame) -> JsResult<()> {
        if self.frames.len() >= self.max_depth {
            return Err(JsException::StackOverflow);
        }
        self.frames.push(frame);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<frame::CallFrame> { self.frames.pop() }
    pub fn current(&self)     -> Option<&frame::CallFrame>     { self.frames.last() }
    pub fn current_mut(&mut self) -> Option<&mut frame::CallFrame> { self.frames.last_mut() }
}

impl Default for CallStack { fn default() -> Self { Self::new() } }
