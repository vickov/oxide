//! Module 6 — Bytecode Compiler
//! Transforms oxc semantic AST into register-based bytecode (~45 opcodes).
//! Register-based (like V8 Ignition) — more efficient than stack-based (JVM).

pub mod opcode;

pub use opcode::Op;

/// A compiled function body — stored in the bytecode cache.
pub struct Bytecode {
    pub ops:        Vec<Op>,
    pub constants:  Vec<crate::heap::value::JsValue>,
    pub formal_args: u32,
    pub name:       Option<crate::heap::value::StringId>,
}

/// Bytecode compiler stub — wires oxc AST output to our bytecode IR.
/// Full implementation: Module 6, ~4 weeks.
pub struct Compiler {
    bytecode_cache: Vec<Bytecode>,
}

impl Compiler {
    pub fn new() -> Self { Self { bytecode_cache: Vec::new() } }
}

impl Default for Compiler { fn default() -> Self { Self::new() } }
