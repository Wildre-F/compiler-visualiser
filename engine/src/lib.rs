//! Compiler-visualizer engine.
//!
//! Pure, deterministic core: source text in → every compilation artifact out
//! (tokens, AST, assembly, machine code), all linked by provenance spans,
//! plus a step-able RISC-V CPU simulator.

pub mod assembler;
pub mod ast;
pub mod codegen;
pub mod cpu;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod vm;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use span::CompileError;
pub use vm::{compile, Compilation, Vm};
