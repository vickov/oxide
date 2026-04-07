//! Module 6 -- Bytecode Compiler
//! Transforms oxc semantic AST into register-based bytecode.
//! Register-based (like V8 Ignition) -- more efficient than stack-based JVM.

pub mod opcode;
pub mod codegen;

pub use opcode::Op;

use crate::heap::value::StringId;

/// A compiled function body.
pub struct Bytecode {
    pub ops:         Vec<Op>,
    pub constants:   Vec<crate::heap::value::JsValue>,
    pub formal_args: u32,
    pub name:        Option<StringId>,
}

pub type BytecodeId = u32;

/// Compile a JS source string and store the resulting Bytecode in the heap.
/// Returns the BytecodeId for the top-level script function.
#[cfg(feature = "parser")]
pub fn compile_script(
    source: &str,
    heap:   &mut crate::heap::JsHeap,
) -> crate::vm::exception::JsResult<BytecodeId> {
    codegen::compile_script_inner(source, heap)
}

/// Evaluate a compiled script on the given engine.
pub fn run_script(
    source:  &str,
    engine:  &mut crate::JsEngine,
) -> crate::vm::exception::JsResult<crate::heap::value::JsValue> {
    #[cfg(feature = "parser")]
    {
        let bid = compile_script(source, &mut engine.heap)?;
        let bytecode = engine.heap.bytecodes[bid as usize].ops.clone();
        let _constants = engine.heap.bytecodes[bid as usize].constants.clone();
        let mut frame = crate::vm::frame::CallFrame::new(bid, 64, crate::heap::value::UNDEFINED);
        // Pre-load constants into constant register space (noop for now -- constants via LoadConst)
        crate::vm::eval::eval(&mut engine.heap, &mut frame, &bytecode, &mut engine.microtasks)
    }
    #[cfg(not(feature = "parser"))]
    {
        Err(crate::vm::exception::JsException::Internal(
            "compile_script requires feature = parser".into()
        ))
    }
}
