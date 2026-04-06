//! Bytecode instruction set — ~45 opcodes, register-based.
//! All instructions reference registers by index (u8).

pub type Reg = u8;
pub type ConstId = u16;
pub type NameId = u32;  // StringId as u32 for compactness

/// Bytecode opcode — one instruction in the VM eval loop.
#[derive(Debug, Clone)]
pub enum Op {
    // --- Values ---
    LoadConst   { dst: Reg, const_id: ConstId },
    LoadUndef   { dst: Reg },
    LoadNull    { dst: Reg },
    LoadBool    { dst: Reg, val: bool },
    LoadInt     { dst: Reg, val: i32 },

    // --- Variables ---
    LoadVar      { dst: Reg, name: NameId },
    StoreVar     { name: NameId, src: Reg },
    LoadCaptured { dst: Reg, cell_idx: u8 },
    StoreCaptured{ cell_idx: u8, src: Reg },

    // --- Objects ---
    NewObject  { dst: Reg },
    NewArray   { dst: Reg, count: u16 },
    GetProp    { dst: Reg, obj: Reg, key: Reg },
    SetProp    { obj: Reg, key: Reg, src: Reg },
    GetPropStr { dst: Reg, obj: Reg, name: NameId },   // fast path — known key
    DeleteProp { obj: Reg, key: Reg },

    // --- Functions ---
    Call        { dst: Reg, func: Reg, this: Reg, argc: u8 },
    CallMethod  { dst: Reg, obj: Reg, method: NameId, argc: u8 },
    NewClosure  { dst: Reg, bytecode_id: u32, capture_count: u8 },
    NewClass    { dst: Reg, ctor_id: u32, method_count: u8 },
    Return      { src: Reg },
    Throw       { src: Reg },

    // --- Control ---
    Jump        { offset: i32 },
    JumpIfTrue  { src: Reg, offset: i32 },
    JumpIfFalse { src: Reg, offset: i32 },

    // --- Arithmetic ---
    Add { dst: Reg, lhs: Reg, rhs: Reg },
    Sub { dst: Reg, lhs: Reg, rhs: Reg },
    Mul { dst: Reg, lhs: Reg, rhs: Reg },
    Div { dst: Reg, lhs: Reg, rhs: Reg },
    Mod { dst: Reg, lhs: Reg, rhs: Reg },
    Neg { dst: Reg, src: Reg },
    Inc { dst: Reg },
    Dec { dst: Reg },

    // --- Comparison ---
    Eq       { dst: Reg, lhs: Reg, rhs: Reg },
    StrictEq { dst: Reg, lhs: Reg, rhs: Reg },
    Lt  { dst: Reg, lhs: Reg, rhs: Reg },
    Lte { dst: Reg, lhs: Reg, rhs: Reg },
    Gt  { dst: Reg, lhs: Reg, rhs: Reg },
    Gte { dst: Reg, lhs: Reg, rhs: Reg },

    // --- Logical ---
    Not      { dst: Reg, src: Reg },
    And      { dst: Reg, lhs: Reg, rhs: Reg },
    Or       { dst: Reg, lhs: Reg, rhs: Reg },
    Coalesce { dst: Reg, lhs: Reg, rhs: Reg },

    // --- Bitwise ---
    BitAnd { dst: Reg, lhs: Reg, rhs: Reg },
    BitOr  { dst: Reg, lhs: Reg, rhs: Reg },
    BitXor { dst: Reg, lhs: Reg, rhs: Reg },
    BitNot { dst: Reg, src: Reg },
    Shl    { dst: Reg, lhs: Reg, rhs: Reg },
    Shr    { dst: Reg, lhs: Reg, rhs: Reg },
    Ushr   { dst: Reg, lhs: Reg, rhs: Reg },

    // --- Exception ---
    TryBegin   { catch_offset: i32, finally_offset: i32 },
    TryEnd,
    EnterCatch { dst: Reg },

    // --- Async ---
    Await { dst: Reg, src: Reg },
    Yield { dst: Reg, src: Reg },
}
