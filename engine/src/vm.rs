//! The top-level engine API: `compile()` → all artifacts, `Vm` → execution.
//!
//! This is the "chunky, not chatty" boundary from the design doc: one big
//! payload at compile time, one small delta per executed cycle.

use crate::assembler::encode;
use crate::ast::{NodeId, Stmt};
use crate::codegen::{codegen, VarSlot};
use crate::cpu::{Cpu, StepDelta};
use crate::lexer::{lex, Token};
use crate::parser::parse;
use crate::span::{CompileError, Span};
use serde::Serialize;

/// One row of the assembly/machine-code listing, with its provenance links.
#[derive(Debug, Clone, Serialize)]
pub struct AsmRow {
    /// Address of this instruction (== index * 4; code is loaded at 0).
    pub addr: u32,
    /// The actual 32-bit machine word.
    pub word: u32,
    /// Human-readable assembly text.
    pub asm: String,
    /// AST node this instruction came from (`None` for prologue/epilogue).
    pub node: Option<NodeId>,
    /// Source span this instruction traces back to.
    pub span: Option<Span>,
}

/// Everything `compile()` produces, in one payload.
#[derive(Debug, Serialize)]
pub struct Compilation {
    pub tokens: Vec<Token>,
    pub ast: Vec<Stmt>,
    pub instrs: Vec<AsmRow>,
    /// Variable slots in the data section.
    pub vars: Vec<VarSlot>,
    /// Where the data section starts (== code size in bytes).
    pub data_base: u32,
}

impl Compilation {
    /// The raw program bytes (little-endian words), as loaded into memory.
    pub fn bytes(&self) -> Vec<u8> {
        self.instrs
            .iter()
            .flat_map(|row| row.word.to_le_bytes())
            .collect()
    }
}

/// Source text → every compilation artifact, linked by provenance.
pub fn compile(src: &str) -> Result<Compilation, CompileError> {
    let tokens = lex(src)?;
    let ast = parse(&tokens)?;
    let out = codegen(&ast)?;

    let instrs = out
        .instrs
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let addr = (i * 4) as u32;
            AsmRow {
                addr,
                word: encode(e.instr),
                asm: e.instr.text(addr),
                node: e.node,
                span: e.span,
            }
        })
        .collect();

    Ok(Compilation {
        tokens,
        ast,
        instrs,
        vars: out.vars,
        data_base: out.data_base,
    })
}

/// A run-able VM: the CPU plus the program needed to (re)load it.
pub struct Vm {
    cpu: Cpu,
    program: Vec<u8>,
    /// instruction index → source span (provenance for the executing PC)
    spans: Vec<Option<Span>>,
}

/// Snapshot of the full CPU state, for the UI's initial paint and reset.
#[derive(Debug, Serialize)]
pub struct VmState {
    pub regs: [i32; 32],
    pub pc: u32,
    pub halted: bool,
    /// The data section's current contents, one word per variable slot.
    pub var_values: Vec<i32>,
}

/// A `StepDelta` enriched with the source span of the executed instruction.
#[derive(Debug, Serialize)]
pub struct TracedDelta {
    #[serde(flatten)]
    pub delta: StepDelta,
    pub span: Option<Span>,
}

impl Vm {
    pub fn new(compilation: &Compilation) -> Vm {
        let program = compilation.bytes();
        Vm {
            cpu: Cpu::new(&program),
            program,
            spans: compilation.instrs.iter().map(|r| r.span).collect(),
        }
    }

    /// One fetch-decode-execute cycle.
    pub fn step(&mut self) -> TracedDelta {
        let delta = self.cpu.step();
        let span = self
            .spans
            .get(delta.instr_index as usize)
            .copied()
            .flatten();
        TracedDelta { delta, span }
    }

    /// Run until halt or `max_steps`, collecting every delta.
    pub fn run(&mut self, max_steps: u32) -> Vec<TracedDelta> {
        let mut deltas = Vec::new();
        for _ in 0..max_steps {
            if self.cpu.halted {
                break;
            }
            deltas.push(self.step());
        }
        deltas
    }

    pub fn reset(&mut self) {
        self.cpu = Cpu::new(&self.program);
    }

    pub fn halted(&self) -> bool {
        self.cpu.halted
    }

    pub fn state(&self, num_vars: usize) -> VmState {
        let data_base = self.program.len() as u32;
        let var_values = (0..num_vars)
            .map(|i| self.cpu.read_word(data_base + (i * 4) as u32).unwrap_or(0))
            .collect();
        VmState {
            regs: self.cpu.regs,
            pc: self.cpu.pc,
            halted: self.cpu.halted,
            var_values,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flagship end-to-end test from the design doc: fibonacci as a loop,
    /// compiled to RISC-V, executed from raw bytes, output collected.
    #[test]
    fn fibonacci_end_to_end() {
        let src = r#"
            let a = 0;
            let b = 1;
            let n = 10;
            while (n > 0) {
                print(a);
                let t = a + b;
                a = b;
                b = t;
                n = n - 1;
            }
        "#;
        // `let t` inside a loop body redeclares - adjust: declare t up front.
        let src = src.replace("let t = a + b;", "t = a + b;");
        let src = format!("let t = 0;\n{src}");

        let compilation = compile(&src).unwrap();
        let mut vm = Vm::new(&compilation);
        let deltas = vm.run(10_000);

        assert!(vm.halted(), "program did not halt");
        let output: String = deltas.iter().filter_map(|d| d.delta.output.clone()).collect();
        assert_eq!(output, "0\n1\n1\n2\n3\n5\n8\n13\n21\n34\n");

        // no faults along the way
        assert!(deltas.iter().all(|d| d.delta.error.is_none()));

        // every executed instruction that has provenance traces to a real span
        for d in &deltas {
            if let Some(span) = d.span {
                assert!((span.end as usize) <= src.len());
                assert!(span.start < span.end);
            }
        }
    }

    #[test]
    fn countdown_with_if_else() {
        let src = r#"
            let n = 3;
            while (n >= 0) {
                if (n == 0) {
                    print(999);
                } else {
                    print(n);
                }
                n = n - 1;
            }
        "#;
        let compilation = compile(src).unwrap();
        let mut vm = Vm::new(&compilation);
        let deltas = vm.run(10_000);
        let output: String = deltas.iter().filter_map(|d| d.delta.output.clone()).collect();
        assert_eq!(output, "3\n2\n1\n999\n");
    }

    #[test]
    fn reset_restores_initial_state() {
        let compilation = compile("let x = 5; print(x);").unwrap();
        let mut vm = Vm::new(&compilation);
        vm.run(100);
        assert!(vm.halted());
        vm.reset();
        assert!(!vm.halted());
        let st = vm.state(1);
        assert_eq!(st.pc, 0);
        assert_eq!(st.regs, [0; 32]);
        // data section wiped too
        assert_eq!(st.var_values, vec![0]);
        // and it runs again identically
        let deltas = vm.run(100);
        let output: String = deltas.iter().filter_map(|d| d.delta.output.clone()).collect();
        assert_eq!(output, "5\n");
    }

    #[test]
    fn state_exposes_variable_values() {
        let compilation = compile("let x = 11; let y = 22;").unwrap();
        let mut vm = Vm::new(&compilation);
        vm.run(100);
        let st = vm.state(compilation.vars.len());
        assert_eq!(st.var_values, vec![11, 22]);
    }

    #[test]
    fn deep_expressions_error_gracefully() {
        // 8 nested parens exceeds the 7-temp register stack
        let src = "let x = (1+(2+(3+(4+(5+(6+(7+(8+9))))))));";
        let err = compile(src).unwrap_err();
        assert!(err.message.contains("too deeply nested"));
    }

    #[test]
    fn division_by_zero_follows_hardware() {
        let compilation = compile("let a = 7; let b = 0; print(a / b);").unwrap();
        let mut vm = Vm::new(&compilation);
        let deltas = vm.run(100);
        let output: String = deltas.iter().filter_map(|d| d.delta.output.clone()).collect();
        assert_eq!(output, "-1\n"); // RISC-V: division by zero yields -1, no trap
    }
}
