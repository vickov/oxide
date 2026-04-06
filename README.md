# Oxide

**A formally verified JavaScript engine in pure Rust.**

> No C. No GC pauses. No CVEs.

Oxide is a JavaScript engine written entirely in safe Rust, designed for
the COBOS unikernel target and Composure's BrowserIo provider. It provides
provable memory safety guarantees that V8 and SpiderMonkey — written in C++ —
fundamentally cannot make.

## Why Oxide?

| Property | V8 / SpiderMonkey | Oxide |
|---|---|---|
| Language | C++ | Pure Rust |
| Memory safety | CVEs regularly | Statically verified |
| GC soundness | Not formally proven | Kani + Verus proven |
| Stack overflow | Platform panic | JsException |
| no_std support | No | Yes (COBOS target) |
| GC pauses | Stop-the-world | Arena-based, bounded |

## Architecture

`
JS source
    -> oxc_parser      (Rust crate, fastest parser in existence)
    -> oxc_semantic    (Rust crate, scope resolution)
    -> Bytecode IR     (register-based, ~45 opcodes)
    -> Interpreter     (Phase 1)
    -> Cranelift JIT   (Phase 2)
`

## Phases

**Phase 1 — Interpreter, Scope A** (~26 weeks)
Run Composure-generated JS. Target: ~70% Test262 conformance.

**Phase 2 — JIT, Scope A** (~6 weeks)
Cranelift JIT for hot functions. Shares PFCL JIT infrastructure.

**Phase 3 — General Web, Scope B** (~12 weeks)
Full ES2020+ conformance. RegExp, Date, Proxy, eval.

## Modules

| # | Module | Effort |
|---|---|---|
| 1 | Value representation (NaN-boxing) | 1 week |
| 2 | Heap + arena allocator | 2 weeks |
| 3 | Garbage collector (mark + sweep + write barrier) | 3 weeks |
| 4 | Object model + shapes | 2 weeks |
| 5 | JS parser (oxc — external) | 0 weeks |
| 6 | Bytecode compiler | 4 weeks |
| 7 | Bytecode interpreter | 3 weeks |
| 8 | Built-in objects | 3 weeks |
| 9 | Prototype chain | 1 week |
| 10 | Closures + scope chain | 2 weeks |
| 11 | Promise + microtask queue | 2 weeks |
| 12 | Exception handling | 1 week |
| 13 | COBOS IoProvider integration | 2 weeks |
| 14 | Cranelift JIT tier (Phase 2) | 6 weeks |

## Verification

- **MIRI** — undefined behaviour on all exercised paths
- **Kani** — bounded proofs of GC soundness and arena invariants
- **Verus** — full deductive proofs of unsafe GC traversal
- **Test262** — official ECMAScript conformance suite (~50,000 tests)
- **Property tests** — JIT output == interpreter output for all sampled inputs

## License

MIT OR Apache-2.0

## Related

- [Composure](https://github.com/vickov/composure) — pure functional runtime this engine serves
- [JS Engine Design](docs/JS_Engine_Rust_Implementation.md) — full implementation breakdown
