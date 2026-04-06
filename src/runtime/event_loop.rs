//! Module 13 — COBOS IoProvider boundary
//! The JS engine is subordinate to the Composure event loop.
//! PFCL never sees JsHeap, JsObject, or JsValue — only Events and Commands.

use crate::heap::HeapRef;
use crate::vm::exception::JsResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(pub u32);

#[derive(Debug)]
pub enum JsTask {
    EvalScript  { source: String },
    DomEvent    { handler: HeapRef, event_obj: HeapRef },
    TimerFired  { callback: HeapRef },
    ModuleLoad  { specifier: String, source: String },
}

#[derive(Debug)]
pub enum DomCommand {
    SetText      { id: String, text: String },
    SetAttribute { id: String, attr: String, value: String },
    AppendChild  { parent: String, child: String },
    RemoveChild  { parent: String, child: String },
    AddClass     { id: String, class: String },
    RemoveClass  { id: String, class: String },
}

pub struct TimerRegistry {
    next_id: u32,
    timers:  Vec<(TimerId, HeapRef, u32, bool)>,
}

impl TimerRegistry {
    pub fn new() -> Self { Self { next_id: 0, timers: Vec::new() } }

    pub fn set_timeout(&mut self, cb: HeapRef, delay_ms: u32) -> TimerId {
        let id = TimerId(self.next_id); self.next_id += 1;
        self.timers.push((id, cb, delay_ms, false)); id
    }
    pub fn set_interval(&mut self, cb: HeapRef, period_ms: u32) -> TimerId {
        let id = TimerId(self.next_id); self.next_id += 1;
        self.timers.push((id, cb, period_ms, true)); id
    }
    pub fn clear(&mut self, id: TimerId) { self.timers.retain(|t| t.0 != id); }
    pub fn next_deadline(&self) -> Option<std::time::Instant> { None }
}

impl Default for TimerRegistry { fn default() -> Self { Self::new() } }

/// Interface the COBOS IoProvider calls. Full implementation: Module 13, ~2 weeks.
pub trait JsRuntime {
    fn execute_task(&mut self, task: JsTask) -> JsResult<Vec<DomCommand>>;
    fn drain_microtasks(&mut self) -> JsResult<()>;
    fn set_timeout(&mut self, callback: HeapRef, delay_ms: u32) -> TimerId;
    fn set_interval(&mut self, callback: HeapRef, period_ms: u32) -> TimerId;
    fn clear_timer(&mut self, id: TimerId);
    fn next_timer_deadline(&self) -> Option<std::time::Instant>;
}
