//! Oxide — A formally verified JavaScript engine in pure Rust.
//!
//! # Architecture
//!
//! `	ext
//! JS source
//!     -> oxc_parser   (Module 5  — external crate)
//!     -> oxc_semantic (Module 5  — external crate)
//!     -> Bytecode IR  (Module 6  — compiler)
//!     -> Interpreter  (Module 7  — Phase 1)
//!     -> Cranelift    (Module 14 — Phase 2, feature = jit)
//! `
//!
//! # Module Map
//!
//! | Module | Path | Status |
//! |--------|------|--------|
//! | 1 Value representation | heap::value | stub |
//! | 2 Heap + arena | heap | stub |
//! | 3 Garbage collector | gc | stub |
//! | 4 Object model | heap::object | stub |
//! | 6 Bytecode compiler | compiler | stub |
//! | 7 Interpreter | m | stub |
//! | 8 Built-ins | uiltins | stub |
//! | 9 Prototype chain | heap::prototype | stub |
//! | 10 Closures | heap::closure | stub |
//! | 11 Promise + microtasks | untime::promise | stub |
//! | 12 Exceptions | untime::exception | stub |
//! | 13 COBOS integration | untime::event_loop | stub |

#![forbid(unsafe_code)]  // All unsafe is in gc:: only, explicitly allowed there

pub mod heap;
pub mod gc;
pub mod vm;
pub mod compiler;
pub mod builtins;
pub mod runtime;

/// The top-level JS runtime — owned by the COBOS IoProvider.
/// Never appears in PFCL. From PFCL's perspective this struct does not exist.
pub struct JsEngine {
    pub heap:       heap::JsHeap,
    pub microtasks: runtime::promise::MicrotaskQueue,
    pub timers:     runtime::event_loop::TimerRegistry,
}

impl JsEngine {
    pub fn new() -> Self {
        Self {
            heap:       heap::JsHeap::new(),
            microtasks: runtime::promise::MicrotaskQueue::new(),
            timers:     runtime::event_loop::TimerRegistry::new(),
        }
    }
}

impl Default for JsEngine {
    fn default() -> Self { Self::new() }
}
