//! Smoke tests — engine initialises and core types work correctly.

use oxide::heap::value;

#[test]
fn engine_initialises() {
    let engine = oxide::JsEngine::new();
    assert!(engine.microtasks.is_empty());
}

#[test]
fn nan_boxing_roundtrips() {
    assert!(value::is_undefined(value::UNDEFINED));
    assert!(value::is_null(value::NULL));
    assert!(value::is_bool(value::TRUE));
    assert!(value::is_bool(value::FALSE));
    assert_eq!(value::as_bool(value::TRUE),  Some(true));
    assert_eq!(value::as_bool(value::FALSE), Some(false));

    let i = value::from_int(42);
    assert!(value::is_int(i));
    assert_eq!(value::as_int(i), Some(42));

    let i_neg = value::from_int(-1);
    assert_eq!(value::as_int(i_neg), Some(-1));

    let f = value::from_float(3.14);
    assert!(value::is_float(f));
    let back = value::as_float(f).unwrap();
    assert!((back - 3.14).abs() < 1e-10);
}

#[test]
fn string_interner_deduplicates() {
    let mut interner = value::StringInterner::new();
    let id1 = interner.intern("hello");
    let id2 = interner.intern("hello");
    let id3 = interner.intern("world");
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
    assert_eq!(interner.get(id1), "hello");
}

#[test]
fn arena_alloc_free_reuse() {
    use oxide::heap::arena::Arena;
    let mut arena: Arena<u32> = Arena::new();
    let r1 = arena.alloc(10u32);
    let r2 = arena.alloc(20u32);
    assert_eq!(arena.get(r1), Some(&10u32));
    assert_eq!(arena.get(r2), Some(&20u32));
    arena.free(r1);
    assert_eq!(arena.get(r1), None);
    // Free list reuse: next alloc reuses r1's slot
    let r3 = arena.alloc(30u32);
    assert_eq!(r3, r1);
    assert_eq!(arena.get(r3), Some(&30u32));
}

#[test]
fn call_stack_overflow_returns_exception() {
    use oxide::vm::{CallStack, frame::CallFrame};
    use oxide::vm::exception::JsException;
    use oxide::heap::value::UNDEFINED;
    let mut stack = CallStack::new();
    for _ in 0..10_000 {
        stack.push(CallFrame::new(0, 4, UNDEFINED)).unwrap();
    }
    let result = stack.push(CallFrame::new(0, 4, UNDEFINED));
    assert!(matches!(result, Err(JsException::StackOverflow)));
}
