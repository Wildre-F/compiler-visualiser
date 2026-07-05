//! The WASM boundary - thin wasm-bindgen wrappers over the pure engine.
//!
//! Chunky, not chatty: `compile()` ships every artifact in one payload;
//! `Vm.step()` ships one small delta per cycle. No other traffic crosses.

use crate::span::CompileError;
use crate::vm;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Serialize with the JSON-compatible profile: structs (and `serde(flatten)`
/// maps) become plain JS objects rather than ES `Map`s.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&ser)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn err_to_js(err: &CompileError) -> JsValue {
    to_js(err).unwrap_or_else(|_| JsValue::from_str(&err.message))
}

/// Compile source text. Returns the full artifact payload
/// (tokens, AST, assembly rows with words + provenance, variable slots),
/// or throws a `{ message, span }` object on error.
#[wasm_bindgen]
pub fn compile(src: &str) -> Result<JsValue, JsValue> {
    let compilation = vm::compile(src).map_err(|e| err_to_js(&e))?;
    to_js(&compilation)
}

/// A step-able virtual machine for a compiled program.
#[wasm_bindgen]
pub struct Vm {
    inner: vm::Vm,
    num_vars: usize,
}

#[wasm_bindgen]
impl Vm {
    /// Build a VM from source. (Compilation is sub-millisecond, so the UI
    /// calls `compile()` for the artifacts and `new Vm()` for execution.)
    #[wasm_bindgen(constructor)]
    pub fn new(src: &str) -> Result<Vm, JsValue> {
        let compilation = vm::compile(src).map_err(|e| err_to_js(&e))?;
        Ok(Vm {
            num_vars: compilation.vars.len(),
            inner: vm::Vm::new(&compilation),
        })
    }

    /// One fetch-decode-execute cycle. Returns a `TracedDelta`.
    pub fn step(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.step())
    }

    /// Run until halt or `max_steps`; returns the array of deltas.
    pub fn run(&mut self, max_steps: u32) -> Result<JsValue, JsValue> {
        to_js(&self.inner.run(max_steps))
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn halted(&self) -> bool {
        self.inner.halted()
    }

    /// Full CPU snapshot (registers, PC, halted, variable slot values) -
    /// for the initial paint and after reset.
    pub fn state(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.state(self.num_vars))
    }
}
