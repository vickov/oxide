//! Bytecode instruction set — ~45 opcodes, register-based.
//! All instructions reference registers by index (u8).
//!
//! `#[repr(u8)]` lets the dispatch table index directly by discriminant.

pub type Reg = u8;
pub type ConstId = u16;
pub type NameId = u32;  // StringId as u32 for compactness

/// Total number of opcode variants — must equal the last discriminant + 1.
pub const NUM_OPCODES: usize = 57;

/// Bytecode opcode — one instruction in the VM eval loop.
///
/// Every variant has an explicit `u8` discriminant so that
/// `Op::discriminant()` maps 1-to-1 to `DISPATCH_TABLE` indices.
#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Op {
    // --- Values ---
    LoadConst    { dst: Reg, const_id: ConstId } = 0,
    LoadUndef    { dst: Reg }                    = 1,
    LoadNull     { dst: Reg }                    = 2,
    LoadBool     { dst: Reg, val: bool }         = 3,
    LoadInt      { dst: Reg, val: i32 }          = 4,

    // --- Variables ---
    LoadVar       { dst: Reg, name: NameId }  = 5,
    StoreVar      { name: NameId, src: Reg }  = 6,
    LoadCaptured  { dst: Reg, cell_idx: u8 }  = 7,
    StoreCaptured { cell_idx: u8, src: Reg }  = 8,

    // --- Objects ---
    NewObject  { dst: Reg }                          = 9,
    NewArray   { dst: Reg, count: u16 }              = 10,
    GetProp    { dst: Reg, obj: Reg, key: Reg }      = 11,
    SetProp    { obj: Reg, key: Reg, src: Reg }      = 12,
    GetPropStr { dst: Reg, obj: Reg, name: NameId }  = 13,
    DeleteProp { obj: Reg, key: Reg }                = 14,

    // --- Functions ---
    Call       { dst: Reg, func: Reg, this: Reg, argc: u8 } = 15,
    CallMethod { dst: Reg, obj: Reg, method: NameId, argc: u8 } = 16,
    NewClosure { dst: Reg, bytecode_id: u32, capture_count: u8 } = 17,
    NewClass   { dst: Reg, ctor_id: u32, method_count: u8 }      = 18,
    Return     { src: Reg }                                       = 19,
    Throw      { src: Reg }                                       = 20,

    // --- Control ---
    Jump        { offset: i32 }              = 21,
    JumpIfTrue  { src: Reg, offset: i32 }   = 22,
    JumpIfFalse { src: Reg, offset: i32 }   = 23,

    // --- Arithmetic ---
    Add { dst: Reg, lhs: Reg, rhs: Reg } = 24,
    Sub { dst: Reg, lhs: Reg, rhs: Reg } = 25,
    Mul { dst: Reg, lhs: Reg, rhs: Reg } = 26,
    Div { dst: Reg, lhs: Reg, rhs: Reg } = 27,
    Mod { dst: Reg, lhs: Reg, rhs: Reg } = 28,
    Neg { dst: Reg, src: Reg }           = 29,
    Inc { dst: Reg }                     = 30,
    Dec { dst: Reg }                     = 31,

    // --- Comparison ---
    Eq       { dst: Reg, lhs: Reg, rhs: Reg } = 32,
    StrictEq { dst: Reg, lhs: Reg, rhs: Reg } = 33,
    Lt       { dst: Reg, lhs: Reg, rhs: Reg } = 34,
    Lte      { dst: Reg, lhs: Reg, rhs: Reg } = 35,
    Gt       { dst: Reg, lhs: Reg, rhs: Reg } = 36,
    Gte      { dst: Reg, lhs: Reg, rhs: Reg } = 37,

    // --- Logical ---
    Not      { dst: Reg, src: Reg }           = 38,
    And      { dst: Reg, lhs: Reg, rhs: Reg } = 39,
    Or       { dst: Reg, lhs: Reg, rhs: Reg } = 40,
    Coalesce { dst: Reg, lhs: Reg, rhs: Reg } = 41,

    // --- Bitwise ---
    BitAnd { dst: Reg, lhs: Reg, rhs: Reg } = 42,
    BitOr  { dst: Reg, lhs: Reg, rhs: Reg } = 43,
    BitXor { dst: Reg, lhs: Reg, rhs: Reg } = 44,
    BitNot { dst: Reg, src: Reg }           = 45,
    Shl    { dst: Reg, lhs: Reg, rhs: Reg } = 46,
    Shr    { dst: Reg, lhs: Reg, rhs: Reg } = 47,
    Ushr   { dst: Reg, lhs: Reg, rhs: Reg } = 48,

    // --- Exception ---
    TryBegin   { catch_offset: i32, finally_offset: i32 } = 49,
    TryEnd                                                 = 50,
    EnterCatch { dst: Reg }                                = 51,

    // --- Async ---
    Await { dst: Reg, src: Reg } = 52,
    Yield { dst: Reg, src: Reg } = 53,

    InstanceOf { dst: Reg, obj: Reg, ctor: Reg } = 54,
    In         { dst: Reg, key: Reg, obj: Reg }  = 55,

    /// Sentinel — returned when no real opcode matches.
    /// Never emitted by the compiler; used only to fill table slots.
    Nop = 56,
}

impl Op {
    /// Return the `u8` discriminant matching `DISPATCH_TABLE` index.
    #[inline(always)]
    pub fn discriminant(&self) -> u8 {
        // SAFETY: because Op is #[repr(u8)], the first byte of the enum
        // *reference* is the discriminant.  We read it through a raw byte
        // pointer — entirely within the object — which is sound for any
        // #[repr(u8)] enum.
        //
        // The crate forbids `unsafe`, so we implement this with a plain
        // `match` instead.  The compiler optimises this to a single movzx.
        match self {
            Op::LoadConst    { .. } =>  0,
            Op::LoadUndef    { .. } =>  1,
            Op::LoadNull     { .. } =>  2,
            Op::LoadBool     { .. } =>  3,
            Op::LoadInt      { .. } =>  4,
            Op::LoadVar      { .. } =>  5,
            Op::StoreVar     { .. } =>  6,
            Op::LoadCaptured { .. } =>  7,
            Op::StoreCaptured{ .. } =>  8,
            Op::NewObject    { .. } =>  9,
            Op::NewArray     { .. } => 10,
            Op::GetProp      { .. } => 11,
            Op::SetProp      { .. } => 12,
            Op::GetPropStr   { .. } => 13,
            Op::DeleteProp   { .. } => 14,
            Op::Call         { .. } => 15,
            Op::CallMethod   { .. } => 16,
            Op::NewClosure   { .. } => 17,
            Op::NewClass     { .. } => 18,
            Op::Return       { .. } => 19,
            Op::Throw        { .. } => 20,
            Op::Jump         { .. } => 21,
            Op::JumpIfTrue   { .. } => 22,
            Op::JumpIfFalse  { .. } => 23,
            Op::Add          { .. } => 24,
            Op::Sub          { .. } => 25,
            Op::Mul          { .. } => 26,
            Op::Div          { .. } => 27,
            Op::Mod          { .. } => 28,
            Op::Neg          { .. } => 29,
            Op::Inc          { .. } => 30,
            Op::Dec          { .. } => 31,
            Op::Eq           { .. } => 32,
            Op::StrictEq     { .. } => 33,
            Op::Lt           { .. } => 34,
            Op::Lte          { .. } => 35,
            Op::Gt           { .. } => 36,
            Op::Gte          { .. } => 37,
            Op::Not          { .. } => 38,
            Op::And          { .. } => 39,
            Op::Or           { .. } => 40,
            Op::Coalesce     { .. } => 41,
            Op::BitAnd       { .. } => 42,
            Op::BitOr        { .. } => 43,
            Op::BitXor       { .. } => 44,
            Op::BitNot       { .. } => 45,
            Op::Shl          { .. } => 46,
            Op::Shr          { .. } => 47,
            Op::Ushr         { .. } => 48,
            Op::TryBegin     { .. } => 49,
            Op::TryEnd            =>  50,
            Op::EnterCatch   { .. } => 51,
            Op::Await        { .. } => 52,
            Op::Yield        { .. } => 53,
            Op::InstanceOf   { .. } => 54,
            Op::In           { .. } => 55,
            Op::Nop               =>  56,
        }
    }
}
