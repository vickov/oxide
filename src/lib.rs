//! Oxide -- A formally verified JavaScript engine in pure Rust.
//!
//! No C. No GC pauses. No CVEs.
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
//! | 7 Interpreter | vm | stub |
//! | 8 Built-ins | builtins | stub |
//! | 9 Prototype chain | heap::prototype | stub |
//! | 10 Closures | heap::closure | stub |
//! | 11 Promise + microtasks | runtime::promise | stub |
//! | 12 Exceptions | runtime::exception | stub |
//! | 13 COBOS integration | runtime::event_loop | stub |

#![forbid(unsafe_code)]

pub mod heap;
pub mod gc;
pub mod vm;
pub mod compiler;
pub mod builtins;
pub mod runtime;

pub struct JsEngine {
    pub heap:       heap::JsHeap,
    pub microtasks: runtime::promise::MicrotaskQueue,
    pub timers:     runtime::event_loop::TimerRegistry,
}

impl JsEngine {
    pub fn new() -> Self {
        let mut heap = heap::JsHeap::new();
        crate::builtins::register_all(&mut heap);
        Self { heap,

            microtasks: runtime::promise::MicrotaskQueue::new(),
            timers:     runtime::event_loop::TimerRegistry::new(),
        }
    }
}

impl Default for JsEngine {
    fn default() -> Self { Self::new() }
}

impl JsEngine {
    /// Compile and run a JS source string. Returns the script's return value.
    /// Requires feature = "parser" (default).
    #[cfg(feature = "parser")]
    pub fn run(&mut self, source: &str) -> crate::vm::exception::JsResult<crate::heap::value::JsValue> {
        crate::compiler::run_script(source, self)
    }
}
