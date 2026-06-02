//! Codegen: AST → RV32IM instructions, each tagged with the AST node it came
//! from (which in turn carries the source span — the provenance chain).
//!
//! Strategy (deliberately simple, v1):
//! - Each variable gets a 4-byte slot in a data section placed right after the
//!   code. `gp` (x3) is set up in a 2-instruction prologue to point at the
//!   data base, so every variable access is a `lw`/`sw` at `offset(gp)`.
//! - Expressions evaluate on a small "register stack" of temporaries t0-t6.
//! - Programs end with an exit ecall; `print(x)` is a print-int ecall.

use crate::ast::*;
use crate::ir::*;
use crate::span::{CompileError, Span};
use std::collections::HashMap;

/// One emitted instruction plus its provenance back-links.
#[derive(Debug, Clone)]
pub struct Emitted {
    pub instr: Instr,
    /// AST node this instruction was generated from. `None` for the fixed
    /// prologue (gp setup) and epilogue (exit ecall).
    pub node: Option<NodeId>,
    pub span: Option<Span>,
}

/// A variable's slot in the data section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VarSlot {
    pub name: String,
    /// Absolute address of the 4-byte slot.
    pub addr: u32,
}

#[derive(Debug)]
pub struct CodegenOutput {
    pub instrs: Vec<Emitted>,
    pub vars: Vec<VarSlot>,
    /// Total code size in bytes; the data section starts here.
    pub data_base: u32,
}

pub fn codegen(program: &[Stmt]) -> Result<CodegenOutput, CompileError> {
    let mut g = Codegen::default();

    // Prologue: load the data-section base address into gp.
    // Emitted with placeholder immediates, patched once code size is known.
    g.emit(Instr::Lui { rd: GP, imm: 0 }, None, None);
    g.emit(Instr::Addi { rd: GP, rs1: GP, imm: 0 }, None, None);

    for stmt in program {
        g.stmt(stmt)?;
    }

    // Epilogue: exit ecall so execution halts cleanly.
    g.emit(Instr::Addi { rd: A7, rs1: ZERO, imm: SYSCALL_EXIT }, None, None);
    g.emit(Instr::Ecall, None, None);

    g.resolve_fixups();

    // Code layout is final; the data section starts right after the code.
    let data_base = (g.instrs.len() * 4) as u32;
    let (hi, lo) = hi_lo(data_base as i32);
    g.instrs[0].instr = Instr::Lui { rd: GP, imm: hi };
    g.instrs[1].instr = Instr::Addi { rd: GP, rs1: GP, imm: lo };

    let vars = g
        .var_order
        .iter()
        .map(|name| VarSlot {
            name: name.clone(),
            addr: data_base + g.var_slots[name] * 4,
        })
        .collect();

    Ok(CodegenOutput {
        instrs: g.instrs,
        vars,
        data_base,
    })
}

/// Split an absolute value into RISC-V %hi/%lo parts such that
/// `(hi << 12) + sign_extend(lo) == value` (the +0x800 compensates for the
/// sign extension of the low 12 bits in `addi`).
fn hi_lo(value: i32) -> (i32, i32) {
    let hi = (value.wrapping_add(0x800)) >> 12;
    let lo = value.wrapping_sub(hi << 12);
    (hi & 0xfffff, lo)
}

type Label = usize;

#[derive(Default)]
struct Codegen {
    instrs: Vec<Emitted>,
    /// variable name → slot index (slot N lives at data_base + N*4)
    var_slots: HashMap<String, u32>,
    var_order: Vec<String>,
    /// next free temporary in TEMPS (expression register stack pointer)
    temp_sp: usize,
    /// label → instruction index it points at
    label_pos: Vec<Option<usize>>,
    /// (instruction index, label) pairs to backpatch
    fixups: Vec<(usize, Label)>,
}

impl Codegen {
    // ---- emission helpers ----------------------------------------------

    fn emit(&mut self, instr: Instr, node: Option<NodeId>, span: Option<Span>) {
        self.instrs.push(Emitted { instr, node, span });
    }

    fn new_label(&mut self) -> Label {
        self.label_pos.push(None);
        self.label_pos.len() - 1
    }

    /// Bind `label` to the *next* instruction to be emitted.
    fn bind(&mut self, label: Label) {
        self.label_pos[label] = Some(self.instrs.len());
    }

    /// Emit a branch/jump whose offset will be patched to `label` later.
    fn emit_to_label(&mut self, instr: Instr, label: Label, node: NodeId, span: Span) {
        self.fixups.push((self.instrs.len(), label));
        self.emit(instr, Some(node), Some(span));
    }

    fn resolve_fixups(&mut self) {
        for &(idx, label) in &self.fixups {
            let target = self.label_pos[label].expect("unbound label");
            let offset = (target as i32 - idx as i32) * 4;
            match &mut self.instrs[idx].instr {
                Instr::Beq { offset: o, .. }
                | Instr::Bne { offset: o, .. }
                | Instr::Blt { offset: o, .. }
                | Instr::Bge { offset: o, .. }
                | Instr::Jal { offset: o, .. } => *o = offset,
                other => panic!("fixup on non-branch instruction {other:?}"),
            }
        }
    }

    // ---- temporaries (the expression register stack) ---------------------

    fn push_temp(&mut self, span: Span) -> Result<Reg, CompileError> {
        if self.temp_sp >= TEMPS.len() {
            return Err(CompileError::new(
                "expression too deeply nested (ran out of temporary registers)",
                span,
            ));
        }
        let r = TEMPS[self.temp_sp];
        self.temp_sp += 1;
        Ok(r)
    }

    fn pop_temp(&mut self) {
        self.temp_sp -= 1;
    }

    // ---- variables -------------------------------------------------------

    fn declare(&mut self, name: &str, span: Span) -> Result<u32, CompileError> {
        if self.var_slots.contains_key(name) {
            return Err(CompileError::new(
                format!("variable '{name}' is already declared"),
                span,
            ));
        }
        let slot = self.var_order.len() as u32;
        self.var_slots.insert(name.to_string(), slot);
        self.var_order.push(name.to_string());
        Ok(slot)
    }

    fn lookup(&self, name: &str, span: Span) -> Result<u32, CompileError> {
        self.var_slots.get(name).copied().ok_or_else(|| {
            CompileError::new(format!("variable '{name}' is not declared"), span)
        })
    }

    // ---- statements -------------------------------------------------------

    fn stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match &stmt.kind {
            StmtKind::Let { name, init } => {
                let slot = self.declare(name, stmt.span)?;
                let r = self.expr(init)?;
                self.emit(
                    Instr::Sw { rs1: GP, rs2: r, imm: (slot * 4) as i32 },
                    Some(stmt.id),
                    Some(stmt.span),
                );
                self.pop_temp();
            }
            StmtKind::Assign { name, value } => {
                let slot = self.lookup(name, stmt.span)?;
                let r = self.expr(value)?;
                self.emit(
                    Instr::Sw { rs1: GP, rs2: r, imm: (slot * 4) as i32 },
                    Some(stmt.id),
                    Some(stmt.span),
                );
                self.pop_temp();
            }
            StmtKind::Print { value } => {
                let r = self.expr(value)?;
                // a0 = value; a7 = print-int; ecall
                self.emit(
                    Instr::Addi { rd: A0, rs1: r, imm: 0 },
                    Some(stmt.id),
                    Some(stmt.span),
                );
                self.pop_temp();
                self.emit(
                    Instr::Addi { rd: A7, rs1: ZERO, imm: SYSCALL_PRINT_INT },
                    Some(stmt.id),
                    Some(stmt.span),
                );
                self.emit(Instr::Ecall, Some(stmt.id), Some(stmt.span));
            }
            StmtKind::If { cond, then_body, else_body } => {
                let l_else = self.new_label();
                let l_end = self.new_label();
                let r = self.expr(cond)?;
                // condition is 0 or 1 in r — skip the then-branch when false
                self.emit_to_label(
                    Instr::Beq { rs1: r, rs2: ZERO, offset: 0 },
                    l_else,
                    stmt.id,
                    cond.span,
                );
                self.pop_temp();
                for s in then_body {
                    self.stmt(s)?;
                }
                if let Some(else_body) = else_body {
                    self.emit_to_label(
                        Instr::Jal { rd: ZERO, offset: 0 },
                        l_end,
                        stmt.id,
                        stmt.span,
                    );
                    self.bind(l_else);
                    for s in else_body {
                        self.stmt(s)?;
                    }
                    self.bind(l_end);
                } else {
                    self.bind(l_else);
                }
            }
            StmtKind::While { cond, body } => {
                let l_top = self.new_label();
                let l_end = self.new_label();
                self.bind(l_top);
                let r = self.expr(cond)?;
                self.emit_to_label(
                    Instr::Beq { rs1: r, rs2: ZERO, offset: 0 },
                    l_end,
                    stmt.id,
                    cond.span,
                );
                self.pop_temp();
                for s in body {
                    self.stmt(s)?;
                }
                self.emit_to_label(
                    Instr::Jal { rd: ZERO, offset: 0 },
                    l_top,
                    stmt.id,
                    stmt.span,
                );
                self.bind(l_end);
            }
        }
        Ok(())
    }

    // ---- expressions --------------------------------------------------
    //
    // Each expression leaves its result in a freshly pushed temporary and
    // returns that register. The caller pops when done.

    fn expr(&mut self, expr: &Expr) -> Result<Reg, CompileError> {
        match &expr.kind {
            ExprKind::Int { value } => {
                let rd = self.push_temp(expr.span)?;
                self.load_imm(rd, *value, expr.id, expr.span);
                Ok(rd)
            }
            ExprKind::Var { name } => {
                let slot = self.lookup(name, expr.span)?;
                let rd = self.push_temp(expr.span)?;
                self.emit(
                    Instr::Lw { rd, rs1: GP, imm: (slot * 4) as i32 },
                    Some(expr.id),
                    Some(expr.span),
                );
                Ok(rd)
            }
            ExprKind::Unary { op: UnOp::Neg, operand } => {
                let r = self.expr(operand)?;
                // r = 0 - r
                self.emit(
                    Instr::Sub { rd: r, rs1: ZERO, rs2: r },
                    Some(expr.id),
                    Some(expr.span),
                );
                Ok(r)
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let r1 = self.expr(lhs)?;
                let r2 = self.expr(rhs)?;
                let id = Some(expr.id);
                let sp = Some(expr.span);
                // result goes into r1; r2 is popped
                match op {
                    BinOp::Add => self.emit(Instr::Add { rd: r1, rs1: r1, rs2: r2 }, id, sp),
                    BinOp::Sub => self.emit(Instr::Sub { rd: r1, rs1: r1, rs2: r2 }, id, sp),
                    BinOp::Mul => self.emit(Instr::Mul { rd: r1, rs1: r1, rs2: r2 }, id, sp),
                    BinOp::Div => self.emit(Instr::Div { rd: r1, rs1: r1, rs2: r2 }, id, sp),
                    BinOp::Rem => self.emit(Instr::Rem { rd: r1, rs1: r1, rs2: r2 }, id, sp),
                    BinOp::Lt => self.emit(Instr::Slt { rd: r1, rs1: r1, rs2: r2 }, id, sp),
                    BinOp::Gt => self.emit(Instr::Slt { rd: r1, rs1: r2, rs2: r1 }, id, sp),
                    BinOp::Le => {
                        // a <= b  ==  !(b < a)
                        self.emit(Instr::Slt { rd: r1, rs1: r2, rs2: r1 }, id, sp);
                        self.emit(Instr::Xori { rd: r1, rs1: r1, imm: 1 }, id, sp);
                    }
                    BinOp::Ge => {
                        // a >= b  ==  !(a < b)
                        self.emit(Instr::Slt { rd: r1, rs1: r1, rs2: r2 }, id, sp);
                        self.emit(Instr::Xori { rd: r1, rs1: r1, imm: 1 }, id, sp);
                    }
                    BinOp::Eq => {
                        // a == b  ==  (a - b) unsigned< 1
                        self.emit(Instr::Sub { rd: r1, rs1: r1, rs2: r2 }, id, sp);
                        self.emit(Instr::Sltiu { rd: r1, rs1: r1, imm: 1 }, id, sp);
                    }
                    BinOp::Ne => {
                        // a != b  ==  0 unsigned< (a - b)
                        self.emit(Instr::Sub { rd: r1, rs1: r1, rs2: r2 }, id, sp);
                        self.emit(Instr::Sltu { rd: r1, rs1: ZERO, rs2: r1 }, id, sp);
                    }
                }
                self.pop_temp(); // r2
                Ok(r1)
            }
        }
    }

    /// Materialize a 32-bit constant: a single `addi` when it fits in 12 bits,
    /// otherwise `lui` + `addi`.
    fn load_imm(&mut self, rd: Reg, value: i32, node: NodeId, span: Span) {
        if (-2048..=2047).contains(&value) {
            self.emit(
                Instr::Addi { rd, rs1: ZERO, imm: value },
                Some(node),
                Some(span),
            );
        } else {
            let (hi, lo) = hi_lo(value);
            self.emit(Instr::Lui { rd, imm: hi }, Some(node), Some(span));
            if lo != 0 {
                self.emit(Instr::Addi { rd, rs1: rd, imm: lo }, Some(node), Some(span));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn gen(src: &str) -> CodegenOutput {
        codegen(&parse(&lex(src).unwrap()).unwrap()).unwrap()
    }

    #[test]
    fn hi_lo_roundtrips() {
        for v in [0, 1, 0x7ff, 0x800, 0x1000, 123456, -1, -2048, -123456, i32::MAX, i32::MIN] {
            let (hi, lo) = hi_lo(v);
            // reconstruct the way lui+addi would
            let got = ((hi << 12) as i32).wrapping_add(lo);
            assert_eq!(got, v, "hi_lo failed for {v}");
            assert!((-2048..=2047).contains(&lo), "lo out of addi range for {v}");
        }
    }

    #[test]
    fn let_compiles_to_store() {
        let out = gen("let x = 7;");
        // prologue (2) + addi t0,zero,7 + sw t0,0(gp) + epilogue (2)
        assert_eq!(out.instrs.len(), 6);
        assert_eq!(out.instrs[2].instr, Instr::Addi { rd: 5, rs1: 0, imm: 7 });
        assert_eq!(out.instrs[3].instr, Instr::Sw { rs1: GP, rs2: 5, imm: 0 });
        assert_eq!(out.vars.len(), 1);
        assert_eq!(out.vars[0].name, "x");
        assert_eq!(out.vars[0].addr, out.data_base);
    }

    #[test]
    fn every_body_instruction_has_provenance() {
        let out = gen("let x = 1; while (x < 5) { x = x + 1; } print(x);");
        // all instructions except the 2-instr prologue and 2-instr epilogue
        let body = &out.instrs[2..out.instrs.len() - 2];
        assert!(!body.is_empty());
        for e in body {
            assert!(e.node.is_some(), "missing node provenance: {:?}", e.instr);
            assert!(e.span.is_some(), "missing span provenance: {:?}", e.instr);
        }
    }

    #[test]
    fn while_branches_resolve() {
        let out = gen("let i = 0; while (i < 3) { i = i + 1; }");
        // find the beq and the jal; their offsets must be non-zero and word-aligned
        let mut saw_beq = false;
        let mut saw_jal = false;
        for e in &out.instrs {
            match e.instr {
                Instr::Beq { offset, .. } => {
                    saw_beq = true;
                    assert!(offset > 0 && offset % 4 == 0, "beq offset {offset}");
                }
                Instr::Jal { rd: 0, offset } => {
                    saw_jal = true;
                    assert!(offset < 0 && offset % 4 == 0, "jal offset {offset}");
                }
                _ => {}
            }
        }
        assert!(saw_beq && saw_jal);
    }

    #[test]
    fn undeclared_variable_is_an_error() {
        let toks = lex("x = 1;").unwrap();
        let ast = parse(&toks).unwrap();
        let err = codegen(&ast).unwrap_err();
        assert!(err.message.contains("not declared"));
    }

    #[test]
    fn redeclaration_is_an_error() {
        let toks = lex("let x = 1; let x = 2;").unwrap();
        let ast = parse(&toks).unwrap();
        let err = codegen(&ast).unwrap_err();
        assert!(err.message.contains("already declared"));
    }

    #[test]
    fn gp_prologue_points_past_code() {
        let out = gen("let x = 42; print(x);");
        let Instr::Lui { rd: GP, imm: hi } = out.instrs[0].instr else {
            panic!("expected lui gp prologue");
        };
        let Instr::Addi { rd: GP, imm: lo, .. } = out.instrs[1].instr else {
            panic!("expected addi gp prologue");
        };
        let base = ((hi << 12) as i32).wrapping_add(lo) as u32;
        assert_eq!(base, out.data_base);
        assert_eq!(base, (out.instrs.len() * 4) as u32);
    }
}
