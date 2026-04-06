//! Module 11 — Promise + Microtask Queue
//! async/await desugars to Promise chains at the bytecode compiler level.

use crate::heap::value::JsValue;
use crate::heap::HeapRef;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum Microtask {
    PromiseReaction { handler: HeapRef, value: JsValue },
    QueuedCallback  { func: HeapRef, args: Vec<JsValue> },
}

pub struct MicrotaskQueue {
    queue: VecDeque<Microtask>,
}

impl MicrotaskQueue {
    pub fn new() -> Self { Self { queue: VecDeque::new() } }

    pub fn enqueue(&mut self, task: Microtask) { self.queue.push_back(task); }
    pub fn pop(&mut self) -> Option<Microtask> { self.queue.pop_front() }
    pub fn is_empty(&self) -> bool             { self.queue.is_empty() }

    /// Drain all microtasks after every JS task.
    /// Full implementation: Module 11, ~2 weeks.
    pub fn drain(&mut self, _heap: &mut crate::heap::JsHeap) {
        // TODO: execute each task, handle new tasks enqueued during execution
        self.queue.clear();
    }
}

impl Default for MicrotaskQueue { fn default() -> Self { Self::new() } }
