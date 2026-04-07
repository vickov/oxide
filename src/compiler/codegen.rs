//! Bytecode code generation -- AST visitor that emits Op instructions.
//! One FnCtx per function body. Scopes are a stack of (name, Reg) tables.

#![cfg(feature = "parser")]

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::heap::{JsHeap, value, value::StringId};
use crate::vm::exception::{JsException, JsResult};
use super::{Bytecode, BytecodeId};
use super::opcode::{Op, Reg, NameId};

// ---------------------------------------------------------------------------
// Name helpers
// ---------------------------------------------------------------------------

fn nid(heap: &mut JsHeap, s: &str) -> NameId { heap.strings.intern(s).0 }

// ---------------------------------------------------------------------------
// Function compilation context
// ---------------------------------------------------------------------------

struct FnCtx {
    ops:      Vec<Op>,
    consts:   Vec<crate::heap::value::JsValue>,
    scopes:   Vec<Vec<(String, Reg)>>,  // scope stack
    next_reg: u8,
    formal_args: u32,
    name:     Option<StringId>,
    // Loop break/continue patch lists (one entry per nested loop)
    breaks:    Vec<Vec<usize>>,
    continues: Vec<Vec<usize>>,
}

impl FnCtx {
    fn new(formal_args: u32, name: Option<StringId>) -> Self {
        Self {
            ops: Vec::new(),
            consts: Vec::new(),
            scopes: vec![Vec::new()],
            next_reg: formal_args as u8,
            formal_args,
            name,
            breaks: Vec::new(),
            continues: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Reg {
        let r = self.next_reg;
        self.next_reg = r.saturating_add(1);
        r
    }

    fn emit(&mut self, op: Op) -> usize {
        let i = self.ops.len();
        self.ops.push(op);
        i
    }

    fn ip(&self) -> usize { self.ops.len() }

    fn add_const(&mut self, v: crate::heap::value::JsValue) -> u16 {
        for (i, &c) in self.consts.iter().enumerate() {
            if c == v { return i as u16; }
        }
        let i = self.consts.len() as u16;
        self.consts.push(v);
        i
    }

    fn push_scope(&mut self) { self.scopes.push(Vec::new()); }
    fn pop_scope(&mut self)  { self.scopes.pop(); }

    fn define(&mut self, name: &str, reg: Reg) {
        if let Some(s) = self.scopes.last_mut() { s.push((name.into(), reg)); }
    }

    fn lookup(&self, name: &str) -> Option<Reg> {
        for scope in self.scopes.iter().rev() {
            for (n, r) in scope.iter().rev() {
                if n == name { return Some(*r); }
            }
        }
        None
    }

    fn patch(&mut self, idx: usize) {
        let target = self.ops.len() as i32;
        let offset = target - idx as i32;
        match &mut self.ops[idx] {
            Op::Jump { offset: o } |
            Op::JumpIfTrue  { offset: o, .. } |
            Op::JumpIfFalse { offset: o, .. } => { *o = offset; }
            _ => {}
        }
    }

    fn patch_all(&mut self, idxs: Vec<usize>) {
        for i in idxs { self.patch(i); }
    }

    fn finish(self) -> Bytecode {
        Bytecode { ops: self.ops, constants: self.consts,
                   formal_args: self.formal_args, name: self.name }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn compile_script_inner(source: &str, heap: &mut JsHeap) -> JsResult<BytecodeId> {
    let alloc = Allocator::default();
    let stype = SourceType::default();
    let ret   = Parser::new(&alloc, source, stype).parse();

    if !ret.errors.is_empty() {
        return Err(JsException::Internal(format!("SyntaxError: {}", ret.errors[0])));
    }

    let mut ctx = FnCtx::new(0, None);
    for stmt in &ret.program.body {
        compile_stmt(&mut ctx, heap, stmt)?;
    }
    let r = ctx.alloc();
    ctx.emit(Op::LoadUndef { dst: r });
    ctx.emit(Op::Return    { src: r });

    let id = heap.bytecodes.len() as u32;
    heap.bytecodes.push(ctx.finish());
    Ok(id)
}

fn compile_function(heap: &mut JsHeap, func: &Function<'_>) -> JsResult<BytecodeId> {
    let formal_args = func.params.items.len() as u32;
    let fname = func.id.as_ref().map(|id| heap.strings.intern(id.name.as_str()));
    let mut ctx = FnCtx::new(formal_args, fname);

    // Bind parameters: args arrive in registers 0..formal_args
    // Store them into the locals HashMap so LoadVar works
    for (i, param) in func.params.items.iter().enumerate() {
        if let BindingPatternKind::BindingIdentifier(bid) = &param.pattern.kind {
            let name = bid.name.as_str();
            let reg  = i as Reg;
            let name_id = nid(heap, name);
            ctx.emit(Op::StoreVar { name: name_id, src: reg });
            ctx.define(name, reg);
        }
    }

    if let Some(body) = &func.body {
        for stmt in &body.statements { compile_stmt(&mut ctx, heap, stmt)?; }
    }

    // Implicit undefined return
    let r = ctx.alloc();
    ctx.emit(Op::LoadUndef { dst: r });
    ctx.emit(Op::Return    { src: r });

    let id = heap.bytecodes.len() as u32;
    heap.bytecodes.push(ctx.finish());
    Ok(id)
}

// ---------------------------------------------------------------------------
// Statement compiler
// ---------------------------------------------------------------------------

fn compile_stmt(ctx: &mut FnCtx, heap: &mut JsHeap, stmt: &Statement) -> JsResult<()> {
    match stmt {
        Statement::ExpressionStatement(es) => {
            compile_expr(ctx, heap, &es.expression)?;
        }
        Statement::BlockStatement(block) => {
            ctx.push_scope();
            for s in &block.body { compile_stmt(ctx, heap, s)?; }
            ctx.pop_scope();
        }
        Statement::VariableDeclaration(decl) => {
            compile_var_decl(ctx, heap, decl)?;
        }
        Statement::FunctionDeclaration(func) => {
            compile_fn_decl(ctx, heap, func)?;
        }
        Statement::ReturnStatement(ret) => {
            let r = if let Some(arg) = &ret.argument {
                compile_expr(ctx, heap, arg)?
            } else {
                let r = ctx.alloc(); ctx.emit(Op::LoadUndef { dst: r }); r
            };
            ctx.emit(Op::Return { src: r });
        }
        Statement::ThrowStatement(thr) => {
            let r = compile_expr(ctx, heap, &thr.argument)?;
            ctx.emit(Op::Throw { src: r });
        }
        Statement::IfStatement(ifs) => {
            compile_if(ctx, heap, ifs)?;
        }
        Statement::WhileStatement(ws) => {
            compile_while(ctx, heap, ws)?;
        }
        Statement::ForStatement(fs) => {
            compile_for(ctx, heap, fs)?;
        }
        Statement::ForInStatement(fis) => {
            // Stub: just evaluate the iterable for side effects
            compile_expr(ctx, heap, &fis.right)?;
        }
        Statement::ForOfStatement(fos) => {
            compile_expr(ctx, heap, &fos.right)?;
        }
        Statement::TryStatement(ts) => {
            compile_try(ctx, heap, ts)?;
        }
        Statement::BreakStatement(_) => {
            let patch = ctx.emit(Op::Jump { offset: 0 });
            if let Some(list) = ctx.breaks.last_mut() { list.push(patch); }
        }
        Statement::ContinueStatement(_) => {
            let patch = ctx.emit(Op::Jump { offset: 0 });
            if let Some(list) = ctx.continues.last_mut() { list.push(patch); }
        }
        Statement::EmptyStatement(_) | Statement::DebuggerStatement(_) => {}
        _ => {} // Ignore unsupported statements (e.g., import/export, TS-specific)
    }
    Ok(())
}

fn compile_var_decl(ctx: &mut FnCtx, heap: &mut JsHeap, decl: &VariableDeclaration) -> JsResult<()> {
    for d in &decl.declarations {
        let reg = ctx.alloc();
        if let Some(init) = &d.init {
            let src = compile_expr(ctx, heap, init)?;
            // Move src into reg if different (simple copy via arithmetic identity)
            if src != reg {
                let tmp = reg;
                ctx.emit(Op::LoadUndef { dst: tmp });
                // Actually just reuse src -- adjust define to point at src
                if let BindingPatternKind::BindingIdentifier(bid) = &d.id.kind {
                    let name = bid.name.as_str();
                    let name_id = nid(heap, name);
                    ctx.emit(Op::StoreVar { name: name_id, src });
                    ctx.define(name, src);
                }
                continue;
            }
        } else {
            ctx.emit(Op::LoadUndef { dst: reg });
        }
        if let BindingPatternKind::BindingIdentifier(bid) = &d.id.kind {
            let name = bid.name.as_str();
            let name_id = nid(heap, name);
            ctx.emit(Op::StoreVar { name: name_id, src: reg });
            ctx.define(name, reg);
        }
    }
    Ok(())
}

fn compile_fn_decl(ctx: &mut FnCtx, heap: &mut JsHeap, func: &Function) -> JsResult<()> {
    let bid = compile_function(heap, func)?;
    let reg = ctx.alloc();
    ctx.emit(Op::NewClosure { dst: reg, bytecode_id: bid, capture_count: 0 });
    if let Some(id) = &func.id {
        let name = id.name.as_str();
        let name_id = nid(heap, name);
        ctx.emit(Op::StoreVar { name: name_id, src: reg });
        ctx.define(name, reg);
    }
    Ok(())
}

fn compile_if(ctx: &mut FnCtx, heap: &mut JsHeap, ifs: &IfStatement) -> JsResult<()> {
    let cond = compile_expr(ctx, heap, &ifs.test)?;
    let jf_patch = ctx.emit(Op::JumpIfFalse { src: cond, offset: 0 });

    compile_stmt(ctx, heap, &ifs.consequent)?;

    if let Some(alt) = &ifs.alternate {
        let jump_patch = ctx.emit(Op::Jump { offset: 0 });
        ctx.patch(jf_patch);
        compile_stmt(ctx, heap, alt)?;
        ctx.patch(jump_patch);
    } else {
        ctx.patch(jf_patch);
    }
    Ok(())
}

fn compile_while(ctx: &mut FnCtx, heap: &mut JsHeap, ws: &WhileStatement) -> JsResult<()> {
    let loop_start = ctx.ip() as i32;
    let cond = compile_expr(ctx, heap, &ws.test)?;
    let exit_patch = ctx.emit(Op::JumpIfFalse { src: cond, offset: 0 });

    ctx.breaks.push(Vec::new());
    ctx.continues.push(Vec::new());

    compile_stmt(ctx, heap, &ws.body)?;

    // Back-edge: jump to loop_start
    let _here = ctx.ip() as i32;
    ctx.emit(Op::Jump { offset: loop_start - here });

    ctx.patch(exit_patch);

    let breaks    = ctx.breaks.pop().unwrap_or_default();
    let continues = ctx.continues.pop().unwrap_or_default();
    ctx.patch_all(breaks);
    // continues jump back to test
    for ci in continues {
        let _here = ctx.ip() as i32;
        if let Op::Jump { offset: o } = &mut ctx.ops[ci] {
            *o = loop_start - ci as i32;
        }
    }
    Ok(())
}

fn compile_for(ctx: &mut FnCtx, heap: &mut JsHeap, fs: &ForStatement) -> JsResult<()> {
    ctx.push_scope();
    // Init
    if let Some(init) = &fs.init {
        match init {
            ForStatementInit::VariableDeclaration(decl) => compile_var_decl(ctx, heap, decl)?,
            _ => { if let Some(e) = init.as_expression() { compile_expr(ctx, heap, e)?; } }
        }
    }
    let loop_start = ctx.ip() as i32;
    // Test
    let exit_patch = if let Some(test) = &fs.test {
        let cond = compile_expr(ctx, heap, test)?;
        Some(ctx.emit(Op::JumpIfFalse { src: cond, offset: 0 }))
    } else { None };

    ctx.breaks.push(Vec::new());
    ctx.continues.push(Vec::new());

    compile_stmt(ctx, heap, &fs.body)?;

    // Update
    let update_ip = ctx.ip() as i32;
    if let Some(update) = &fs.update { compile_expr(ctx, heap, update)?; }

    let _here = ctx.ip() as i32;
    ctx.emit(Op::Jump { offset: loop_start - here });

    if let Some(p) = exit_patch { ctx.patch(p); }

    let breaks    = ctx.breaks.pop().unwrap_or_default();
    let continues = ctx.continues.pop().unwrap_or_default();
    ctx.patch_all(breaks);
    for ci in continues {
        if let Op::Jump { offset: o } = &mut ctx.ops[ci] { *o = update_ip - ci as i32; }
    }
    ctx.pop_scope();
    Ok(())
}

fn compile_try(ctx: &mut FnCtx, heap: &mut JsHeap, ts: &TryStatement) -> JsResult<()> {
    // Emit TryBegin with placeholder offsets
    let try_patch = ctx.emit(Op::TryBegin { catch_offset: 0, finally_offset: 0 });

    // Try block
    ctx.push_scope();
    for s in &ts.block.body { compile_stmt(ctx, heap, s)?; }
    ctx.pop_scope();
    ctx.emit(Op::TryEnd);

    // Jump over catch block (normal exit)
    let over_catch = ctx.emit(Op::Jump { offset: 0 });

    // Patch TryBegin catch_offset to point here
    let catch_ip = ctx.ip() as i32;
    if let Op::TryBegin { catch_offset, .. } = &mut ctx.ops[try_patch] {
        *catch_offset = catch_ip - try_patch as i32;
    }

    // Catch block
    if let Some(handler) = &ts.handler {
        let catch_reg = ctx.alloc();
        ctx.emit(Op::EnterCatch { dst: catch_reg });
        if let Some(param) = &handler.param {
            if let BindingPatternKind::BindingIdentifier(bid) = &param.pattern.kind {
                let name = bid.name.as_str();
                let name_id = nid(heap, name);
                ctx.emit(Op::StoreVar { name: name_id, src: catch_reg });
                ctx.define(name, catch_reg);
            }
        }
        ctx.push_scope();
        for s in &handler.body.body { compile_stmt(ctx, heap, s)?; }
        ctx.pop_scope();
    }

    ctx.patch(over_catch);

    // Finally block
    if let Some(fin) = &ts.finalizer {
        for s in &fin.body { compile_stmt(ctx, heap, s)?; }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Expression compiler -- returns the register holding the result
// ---------------------------------------------------------------------------

fn compile_expr(ctx: &mut FnCtx, heap: &mut JsHeap, expr: &Expression) -> JsResult<Reg> {
    match expr {
        // Literals
        Expression::BooleanLiteral(b) => {
            let r = ctx.alloc();
            ctx.emit(Op::LoadBool { dst: r, val: b.value });
            Ok(r)
        }
        Expression::NullLiteral(_) => {
            let r = ctx.alloc(); ctx.emit(Op::LoadNull { dst: r }); Ok(r)
        }
        Expression::NumericLiteral(n) => {
            let r = ctx.alloc();
            let f = n.value;
            if f.fract() == 0.0 && f >= i32::MIN as f64 && f <= i32::MAX as f64 {
                ctx.emit(Op::LoadInt { dst: r, val: f as i32 });
            } else {
                let v  = value::from_float(f);
                let ci = ctx.add_const(v);
                ctx.emit(Op::LoadConst { dst: r, const_id: ci });
            }
            Ok(r)
        }
        Expression::StringLiteral(s) => {
            let r   = ctx.alloc();
            let sid = heap.strings.intern(s.value.as_str());
            let v   = value::from_string(sid);
            let ci  = ctx.add_const(v);
            ctx.emit(Op::LoadConst { dst: r, const_id: ci });
            Ok(r)
        }
        Expression::TemplateLiteral(tl) => {
            compile_template(ctx, heap, tl)
        }

        // Identifier reference
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            match name {
                "undefined" => { let r = ctx.alloc(); ctx.emit(Op::LoadUndef { dst: r }); Ok(r) }
                "null"      => { let r = ctx.alloc(); ctx.emit(Op::LoadNull  { dst: r }); Ok(r) }
                "true"      => { let r = ctx.alloc(); ctx.emit(Op::LoadBool  { dst: r, val: true  }); Ok(r) }
                "false"     => { let r = ctx.alloc(); ctx.emit(Op::LoadBool  { dst: r, val: false }); Ok(r) }
                "Infinity"  => {
                    let r = ctx.alloc();
                    let v = value::from_float(f64::INFINITY);
                    let ci = ctx.add_const(v); ctx.emit(Op::LoadConst { dst: r, const_id: ci }); Ok(r)
                }
                "NaN" => {
                    let r = ctx.alloc();
                    let v = value::from_float(f64::NAN);
                    let ci = ctx.add_const(v); ctx.emit(Op::LoadConst { dst: r, const_id: ci }); Ok(r)
                }
                _ => {
                    let r      = ctx.alloc();
                    let name_id = nid(heap, name);
                    ctx.emit(Op::LoadVar { dst: r, name: name_id });
                    Ok(r)
                }
            }
        }

        // this
        Expression::ThisExpression(_) => {
            let r = ctx.alloc();
            ctx.emit(Op::LoadVar { dst: r, name: nid(heap, "this") });
            Ok(r)
        }

        // Parenthesized
        Expression::ParenthesizedExpression(pe) => compile_expr(ctx, heap, &pe.expression),

        // Sequence (a, b, c) -> result of c
        Expression::SequenceExpression(se) => {
            let mut last = ctx.alloc();
            ctx.emit(Op::LoadUndef { dst: last });
            for e in &se.expressions { last = compile_expr(ctx, heap, e)?; }
            Ok(last)
        }

        // Unary
        Expression::UnaryExpression(ue) => compile_unary(ctx, heap, ue),

        // Update (++x, x++, etc.)
        Expression::UpdateExpression(ue) => compile_update(ctx, heap, ue),

        // Binary
        Expression::BinaryExpression(be) => compile_binary(ctx, heap, be),

        // Logical (&& || ??)
        Expression::LogicalExpression(le) => compile_logical(ctx, heap, le),

        // Conditional (ternary)
        Expression::ConditionalExpression(ce) => {
            let cond  = compile_expr(ctx, heap, &ce.test)?;
            let jf    = ctx.emit(Op::JumpIfFalse { src: cond, offset: 0 });
            let then  = compile_expr(ctx, heap, &ce.consequent)?;
            let jover = ctx.emit(Op::Jump { offset: 0 });
            ctx.patch(jf);
            let _alt  = compile_expr(ctx, heap, &ce.alternate)?;
            ctx.patch(jover);
            // Result is in `then` or `alt` -- use then as canonical (may differ if regs diverge)
            Ok(then)
        }

        // Assignment
        Expression::AssignmentExpression(ae) => compile_assign(ctx, heap, ae),

        // Member access: obj.prop
        Expression::StaticMemberExpression(me) => {
            let obj  = compile_expr(ctx, heap, &me.object)?;
            let r    = ctx.alloc();
            let name_id = nid(heap, me.property.name.as_str());
            ctx.emit(Op::GetPropStr { dst: r, obj, name: name_id });
            Ok(r)
        }

        // Member access: obj[expr]
        Expression::ComputedMemberExpression(me) => {
            let obj = compile_expr(ctx, heap, &me.object)?;
            let key = compile_expr(ctx, heap, &me.expression)?;
            let r   = ctx.alloc();
            ctx.emit(Op::GetProp { dst: r, obj, key });
            Ok(r)
        }

        // Function call: callee(args) or obj.method(args)
        Expression::CallExpression(ce) => compile_call(ctx, heap, ce),

        // new Ctor(args)
        Expression::NewExpression(ne) => {
            // Emit as a Call -- proper `new` semantics require more work
            let func = compile_expr(ctx, heap, &ne.callee)?;
            let this = ctx.alloc();
            ctx.emit(Op::NewObject { dst: this });
            let mut argc = 0u8;
            for arg in &ne.arguments {
                if let Some(e) = arg.as_expression() {
                    let _ = compile_expr(ctx, heap, e)?;
                    argc += 1;
                }
            }
            let dst = ctx.alloc();
            ctx.emit(Op::Call { dst, func, this, argc });
            Ok(dst)
        }

        // Object literal: { key: value, ... }
        Expression::ObjectExpression(oe) => compile_object(ctx, heap, oe),

        // Array literal: [a, b, c]
        Expression::ArrayExpression(ae) => compile_array(ctx, heap, ae),

        // Function expression
        Expression::FunctionExpression(func) => {
            let bid = compile_function(heap, func)?;
            let r   = ctx.alloc();
            ctx.emit(Op::NewClosure { dst: r, bytecode_id: bid, capture_count: 0 });
            Ok(r)
        }

        // Arrow function: (a) => expr
        Expression::ArrowFunctionExpression(af) => {
            let formal_args = af.params.items.len() as u32;
            let mut inner = FnCtx::new(formal_args, None);

            // Bind params
            for (i, param) in af.params.items.iter().enumerate() {
                if let BindingPatternKind::BindingIdentifier(bid) = &param.pattern.kind {
                    let name = bid.name.as_str();
                    let name_id = nid(heap, name);
                    inner.emit(Op::StoreVar { name: name_id, src: i as Reg });
                    inner.define(name, i as Reg);
                }
            }

            // Body
            if af.expression {
                // Arrow expression body: `() => expr`
                // The body has one statement which is the expression
                if let Some(stmt) = af.body.statements.first() {
                    if let Statement::ExpressionStatement(es) = stmt {
                        let r = compile_expr(&mut inner, heap, &es.expression)?;
                        inner.emit(Op::Return { src: r });
                    }
                }
            } else {
                for s in &af.body.statements { compile_stmt(&mut inner, heap, s)?; }
                let r = inner.alloc(); inner.emit(Op::LoadUndef { dst: r }); inner.emit(Op::Return { src: r });
            }

            let bid = heap.bytecodes.len() as u32;
            heap.bytecodes.push(inner.finish());
            let r = ctx.alloc();
            ctx.emit(Op::NewClosure { dst: r, bytecode_id: bid, capture_count: 0 });
            Ok(r)
        }

        // Await
        Expression::AwaitExpression(ae) => {
            let src = compile_expr(ctx, heap, &ae.argument)?;
            let dst = ctx.alloc();
            ctx.emit(Op::Await { dst, src });
            Ok(dst)
        }

        // Yield
        Expression::YieldExpression(ye) => {
            let src = if let Some(arg) = &ye.argument {
                compile_expr(ctx, heap, arg)?
            } else { let r = ctx.alloc(); ctx.emit(Op::LoadUndef { dst: r }); r };
            let dst = ctx.alloc();
            ctx.emit(Op::Yield { dst, src });
            Ok(dst)
        }

        // Fallback: emit undefined for unsupported expressions
        _ => {
            let r = ctx.alloc(); ctx.emit(Op::LoadUndef { dst: r }); Ok(r)
        }
    }
}

// ---------------------------------------------------------------------------
// Expression helpers
// ---------------------------------------------------------------------------

fn compile_unary(ctx: &mut FnCtx, heap: &mut JsHeap, ue: &UnaryExpression) -> JsResult<Reg> {
    match ue.operator {
        UnaryOperator::LogicalNot => {
            let src = compile_expr(ctx, heap, &ue.argument)?;
            let dst = ctx.alloc();
            ctx.emit(Op::Not { dst, src }); Ok(dst)
        }
        UnaryOperator::UnaryNegation => {
            let src = compile_expr(ctx, heap, &ue.argument)?;
            let dst = ctx.alloc();
            ctx.emit(Op::Neg { dst, src }); Ok(dst)
        }
        UnaryOperator::BitwiseNot => {
            let src = compile_expr(ctx, heap, &ue.argument)?;
            let dst = ctx.alloc();
            ctx.emit(Op::BitNot { dst, src }); Ok(dst)
        }
        UnaryOperator::Typeof => {
            // typeof returns a string -- stub as undefined for now
            let _ = compile_expr(ctx, heap, &ue.argument)?;
            let r = ctx.alloc();
            let sid = heap.strings.intern("undefined");
            let v = value::from_string(sid);
            let ci = ctx.add_const(v);
            ctx.emit(Op::LoadConst { dst: r, const_id: ci });
            Ok(r)
        }
        UnaryOperator::Void => {
            compile_expr(ctx, heap, &ue.argument)?;
            let r = ctx.alloc(); ctx.emit(Op::LoadUndef { dst: r }); Ok(r)
        }
        _ => { // UnaryPlus, Delete
            let src = compile_expr(ctx, heap, &ue.argument)?;
            Ok(src)
        }
    }
}

fn compile_update(ctx: &mut FnCtx, heap: &mut JsHeap, ue: &UpdateExpression) -> JsResult<Reg> {
    // ue.argument is SimpleAssignmentTarget, not Expression
    // Extract identifier name if available (simple case: ++x, x--, etc.)
    let (reg, name_id_opt) = match &ue.argument {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
            let name = id.name.as_str();
            let n = nid(heap, name);
            let r = ctx.alloc();
            ctx.emit(Op::LoadVar { dst: r, name: n });
            (r, Some(n))
        }
        // Member expressions: load the property value
        SimpleAssignmentTarget::StaticMemberExpression(me) => {
            let obj = compile_expr(ctx, heap, &me.object)?;
            let r = ctx.alloc();
            ctx.emit(Op::GetPropStr { dst: r, obj, name: nid(heap, me.property.name.as_str()) });
            (r, None)
        }
        _ => {
            let r = ctx.alloc(); ctx.emit(Op::LoadUndef { dst: r }); (r, None)
        }
    };

    let dst = reg;
    match ue.operator {
        UpdateOperator::Increment => ctx.emit(Op::Inc { dst }),
        UpdateOperator::Decrement => ctx.emit(Op::Dec { dst }),
    };

    // Sync updated value back to variable
    if let Some(n) = name_id_opt {
        ctx.emit(Op::StoreVar { name: n, src: dst });
    }
    Ok(dst)
}

fn compile_binary(ctx: &mut FnCtx, heap: &mut JsHeap, be: &BinaryExpression) -> JsResult<Reg> {
    let lhs = compile_expr(ctx, heap, &be.left)?;
    let rhs = compile_expr(ctx, heap, &be.right)?;
    let dst = ctx.alloc();
    let op = match be.operator {
        BinaryOperator::Addition              => Op::Add      { dst, lhs, rhs },
        BinaryOperator::Subtraction           => Op::Sub      { dst, lhs, rhs },
        BinaryOperator::Multiplication        => Op::Mul      { dst, lhs, rhs },
        BinaryOperator::Division              => Op::Div      { dst, lhs, rhs },
        BinaryOperator::Remainder             => Op::Mod      { dst, lhs, rhs },
        BinaryOperator::Equality              => Op::Eq       { dst, lhs, rhs },
        BinaryOperator::Inequality            => { ctx.emit(Op::Eq { dst, lhs, rhs }); ctx.emit(Op::Not { dst, src: dst }); return Ok(dst); }
        BinaryOperator::StrictEquality        => Op::StrictEq { dst, lhs, rhs },
        BinaryOperator::StrictInequality      => { ctx.emit(Op::StrictEq { dst, lhs, rhs }); ctx.emit(Op::Not { dst, src: dst }); return Ok(dst); }
        BinaryOperator::LessThan              => Op::Lt       { dst, lhs, rhs },
        BinaryOperator::LessEqualThan         => Op::Lte      { dst, lhs, rhs },
        BinaryOperator::GreaterThan           => Op::Gt       { dst, lhs, rhs },
        BinaryOperator::GreaterEqualThan      => Op::Gte      { dst, lhs, rhs },
        BinaryOperator::BitwiseAnd            => Op::BitAnd   { dst, lhs, rhs },
        BinaryOperator::BitwiseOR             => Op::BitOr    { dst, lhs, rhs },
        BinaryOperator::BitwiseXOR            => Op::BitXor   { dst, lhs, rhs },
        BinaryOperator::ShiftLeft             => Op::Shl      { dst, lhs, rhs },
        BinaryOperator::ShiftRight            => Op::Shr      { dst, lhs, rhs },
        BinaryOperator::ShiftRightZeroFill    => Op::Ushr     { dst, lhs, rhs },
        BinaryOperator::Instanceof            => Op::InstanceOf { dst, obj: lhs, ctor: rhs },
        BinaryOperator::In                    => Op::In       { dst, key: lhs, obj: rhs },
        BinaryOperator::Exponential           => {
            // a ** b -- emit as Math.pow(a, b) stub: just multiply for now
            ctx.emit(Op::Mul { dst, lhs, rhs }); return Ok(dst);
        }
    };
    ctx.emit(op);
    Ok(dst)
}

fn compile_logical(ctx: &mut FnCtx, heap: &mut JsHeap, le: &LogicalExpression) -> JsResult<Reg> {
    let lhs = compile_expr(ctx, heap, &le.left)?;
    let rhs = compile_expr(ctx, heap, &le.right)?;
    let dst = ctx.alloc();
    let op = match le.operator {
        LogicalOperator::And             => Op::And      { dst, lhs, rhs },
        LogicalOperator::Or              => Op::Or       { dst, lhs, rhs },
        LogicalOperator::Coalesce        => Op::Coalesce { dst, lhs, rhs },
    };
    ctx.emit(op);
    Ok(dst)
}

fn compile_assign(ctx: &mut FnCtx, heap: &mut JsHeap, ae: &AssignmentExpression) -> JsResult<Reg> {
    let rhs = compile_expr(ctx, heap, &ae.right)?;

    // Get the target identifier name (simple assignment only for now)
    let target_name: Option<String> = match &ae.left {
        AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.as_str().to_string()),
        _ => None,
    };

    if let Some(name) = &target_name {
        let name_id = nid(heap, name);

        // Compound assignment: load old value, apply op, store result
        let dst = match ae.operator {
            AssignmentOperator::Assign => rhs,
            _ => {
                let old = ctx.alloc();
                ctx.emit(Op::LoadVar { dst: old, name: name_id });
                let result = ctx.alloc();
                let op = match ae.operator {
                    AssignmentOperator::Addition       => Op::Add { dst: result, lhs: old, rhs },
                    AssignmentOperator::Subtraction    => Op::Sub { dst: result, lhs: old, rhs },
                    AssignmentOperator::Multiplication => Op::Mul { dst: result, lhs: old, rhs },
                    AssignmentOperator::Division       => Op::Div { dst: result, lhs: old, rhs },
                    AssignmentOperator::Remainder      => Op::Mod { dst: result, lhs: old, rhs },
                    AssignmentOperator::BitwiseAnd     => Op::BitAnd { dst: result, lhs: old, rhs },
                    AssignmentOperator::BitwiseOR      => Op::BitOr  { dst: result, lhs: old, rhs },
                    AssignmentOperator::BitwiseXOR     => Op::BitXor { dst: result, lhs: old, rhs },
                    AssignmentOperator::ShiftLeft      => Op::Shl { dst: result, lhs: old, rhs },
                    AssignmentOperator::ShiftRight     => Op::Shr { dst: result, lhs: old, rhs },
                    AssignmentOperator::ShiftRightZeroFill => Op::Ushr { dst: result, lhs: old, rhs },
                    AssignmentOperator::LogicalAnd     => Op::And { dst: result, lhs: old, rhs },
                    AssignmentOperator::LogicalOr      => Op::Or  { dst: result, lhs: old, rhs },
                    AssignmentOperator::LogicalNullish => Op::Coalesce { dst: result, lhs: old, rhs },
                    _ => Op::Add { dst: result, lhs: old, rhs }, // Exponentiation fallback
                };
                ctx.emit(op);
                result
            }
        };
        ctx.emit(Op::StoreVar { name: name_id, src: dst });
        // Update local binding register if tracked
        if let Some(local_reg) = ctx.lookup(name) {
            if local_reg != dst {
                ctx.emit(Op::StoreVar { name: name_id, src: dst });
            }
        }
        return Ok(dst);
    }

    // Member assignment: obj.prop = rhs  or  obj[key] = rhs
    match &ae.left {
        AssignmentTarget::StaticMemberExpression(me) => {
            // me: &StaticMemberExpression -- object field is Expression type
            let obj     = compile_expr(ctx, heap, &me.object)?;
            let key_reg = ctx.alloc();
            let sid     = heap.strings.intern(me.property.name.as_str());
            let kv      = value::from_string(sid);
            let ci      = ctx.add_const(kv);
            ctx.emit(Op::LoadConst { dst: key_reg, const_id: ci });
            ctx.emit(Op::SetProp { obj, key: key_reg, src: rhs });
        }
        AssignmentTarget::ComputedMemberExpression(me) => {
            let obj = compile_expr(ctx, heap, &me.object)?;
            let key = compile_expr(ctx, heap, &me.expression)?;
            ctx.emit(Op::SetProp { obj, key, src: rhs });
        }
        _ => {}
    }
    Ok(rhs)
}

fn compile_call(ctx: &mut FnCtx, heap: &mut JsHeap, ce: &CallExpression) -> JsResult<Reg> {
    let dst = ctx.alloc();
    let argc = ce.arguments.iter().filter(|a| a.as_expression().is_some()).count() as u8;

    match &ce.callee {
        // obj.method(args) -> CallMethod
        Expression::StaticMemberExpression(me) => {
            let obj      = compile_expr(ctx, heap, &me.object)?;
            let method   = nid(heap, me.property.name.as_str());
            // Emit args into consecutive registers after obj
            let mut arg_regs: Vec<Reg> = Vec::new();
            for arg in ce.arguments.iter() {
                if let Some(e) = arg.as_expression() {
                    arg_regs.push(compile_expr(ctx, heap, e)?);
                }
            }
            ctx.emit(Op::CallMethod { dst, obj, method, argc });
        }
        // func(args) -> Call
        callee => {
            let func = compile_expr(ctx, heap, callee)?;
            let this = ctx.alloc();
            ctx.emit(Op::LoadUndef { dst: this });
            for arg in ce.arguments.iter() {
                if let Some(e) = arg.as_expression() {
                    compile_expr(ctx, heap, e)?;
                }
            }
            ctx.emit(Op::Call { dst, func, this, argc });
        }
    }
    Ok(dst)
}

fn compile_object(ctx: &mut FnCtx, heap: &mut JsHeap, oe: &ObjectExpression) -> JsResult<Reg> {
    let obj = ctx.alloc();
    ctx.emit(Op::NewObject { dst: obj });
    for prop in &oe.properties {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            // Key
            let key_reg = ctx.alloc();
            let key_str: String = match &p.key {
                PropertyKey::StaticIdentifier(id)  => id.name.as_str().to_string(),
                PropertyKey::StringLiteral(s)       => s.value.as_str().to_string(),
                PropertyKey::NumericLiteral(n)      => n.value.to_string(),
                _ => continue,
            };
            let sid = heap.strings.intern(&key_str);
            let kv  = value::from_string(sid);
            let ci  = ctx.add_const(kv);
            ctx.emit(Op::LoadConst { dst: key_reg, const_id: ci });
            // Value
            let val_reg = compile_expr(ctx, heap, &p.value)?;
            ctx.emit(Op::SetProp { obj, key: key_reg, src: val_reg });
        }
    }
    Ok(obj)
}

fn compile_array(ctx: &mut FnCtx, heap: &mut JsHeap, ae: &ArrayExpression) -> JsResult<Reg> {
    let arr = ctx.alloc();
    let count = ae.elements.len() as u16;
    ctx.emit(Op::NewArray { dst: arr, count });
    for (i, elem) in ae.elements.iter().enumerate() {
        if let Some(e) = elem.as_expression() {
            let val = compile_expr(ctx, heap, e)?;
            // Store as obj["i"] = val
            let key_reg = ctx.alloc();
            let idx_s   = i.to_string();
            let sid     = heap.strings.intern(&idx_s);
            let kv      = value::from_string(sid);
            let ci      = ctx.add_const(kv);
            ctx.emit(Op::LoadConst { dst: key_reg, const_id: ci });
            ctx.emit(Op::SetProp   { obj: arr, key: key_reg, src: val });
        }
    }
    // Set length property
    let len_reg = ctx.alloc();
    ctx.emit(Op::LoadInt { dst: len_reg, val: ae.elements.len() as i32 });
    let lkey_reg = ctx.alloc();
    let lsid = heap.strings.intern("length");
    let lkv  = value::from_string(lsid);
    let lci  = ctx.add_const(lkv);
    ctx.emit(Op::LoadConst { dst: lkey_reg, const_id: lci });
    ctx.emit(Op::SetProp { obj: arr, key: lkey_reg, src: len_reg });
    Ok(arr)
}

fn compile_template(ctx: &mut FnCtx, heap: &mut JsHeap, tl: &TemplateLiteral) -> JsResult<Reg> {
    // Simple template: concatenate quasis and expressions alternately
    let mut result = ctx.alloc();
    let empty_sid = heap.strings.intern("");
    let empty_v   = value::from_string(empty_sid);
    let ci        = ctx.add_const(empty_v);
    ctx.emit(Op::LoadConst { dst: result, const_id: ci });

    for (i, quasi) in tl.quasis.iter().enumerate() {
        // Append the quasi string
        let s = quasi.value.raw.as_str().to_string();
        if !s.is_empty() {
            let sid = heap.strings.intern(&s);
            let v   = value::from_string(sid);
            let ci  = ctx.add_const(v);
            let tmp = ctx.alloc();
            ctx.emit(Op::LoadConst { dst: tmp, const_id: ci });
            let dst = ctx.alloc();
            ctx.emit(Op::Add { dst, lhs: result, rhs: tmp });
            result = dst;
        }
        // Append the expression (if any)
        if i < tl.expressions.len() {
            let expr_reg = compile_expr(ctx, heap, &tl.expressions[i])?;
            let dst = ctx.alloc();
            ctx.emit(Op::Add { dst, lhs: result, rhs: expr_reg });
            result = dst;
        }
    }
    Ok(result)
}
