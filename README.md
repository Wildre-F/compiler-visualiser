# Compiler Visualizer

**Source → tokens → AST → RISC-V assembly → machine code → a stepping CPU — all
synchronized, in your browser.**

Write a small program, watch it compile through every stage, then step a
simulated RV32IM CPU through the actual machine-code bytes one
fetch–decode–execute cycle at a time. Click any line of source and the matching
AST nodes, assembly rows, and bytes light up; step the CPU and the highlight
runs the other way.

```
   Source ──lex──▶ Tokens ──parse──▶ AST ──codegen──▶ RISC-V asm ──assemble──▶ bytes
      ▲                                                                          │
      └────────────────── synchronized highlight ◀────── CPU executes ◀─────────┘
```

## Why it's interesting

- **Provenance end to end.** Every token, AST node, instruction, and byte
  carries a tag pointing back to the source span it came from. That single
  discipline is what makes the synchronized highlight possible.
- **The bytes are real.** Instructions assemble to genuine RV32IM encodings
  (verified against known-good words). The simulator *decodes the actual bytes
  in memory* every cycle — the machine-code panel isn't decoration, it's what
  executes.
- **A clean engine/UI boundary.** The engine is pure Rust compiled to WASM:
  `compile()` returns every artifact in one payload; `vm.step()` returns a
  small state delta per cycle. React renders state and forwards clicks — it
  computes nothing.

## Stack

| Layer  | Tech                                  |
|--------|---------------------------------------|
| Engine | Rust → WebAssembly (wasm-bindgen)     |
| UI     | React + TypeScript + Vite + CodeMirror 6 |
| Deploy | Fully static — no backend             |

## The language

A tiny C-like language: integers, `let`, assignment, arithmetic
(`+ - * / %`), comparisons, `if`/`else`, `while`, and `print()`. Enough for
fibonacci, collatz, and trial-division primes (all included as examples).

`print(x)` compiles to a print-int ecall; programs end with an exit ecall.
Division by zero returns -1 — because that's what the RISC-V spec says real
hardware does.

## Run it

```bash
# engine (needs Rust + wasm-pack)
cd engine && wasm-pack build --target web

# ui
cd ../web && npm install && npm run dev
```

Then open http://localhost:5173.

```bash
# engine test suite (33 tests, incl. fibonacci end-to-end)
cd engine && cargo test
```

## Project layout

```
engine/             pure Rust core (cdylib + rlib)
  src/
    span.rs         Span + CompileError — the provenance backbone
    lexer.rs        text → spanned tokens
    ast.rs          AST node types (id + span on every node)
    parser.rs       recursive descent → AST
    ir.rs           the RV32IM instruction subset
    codegen.rs      AST → instructions (gp-relative variable slots,
                    register-stack expressions, label fixups)
    assembler.rs    instruction ↔ 32-bit word (encode + decode)
    cpu.rs          fetch/decode/execute core, per-cycle deltas
    vm.rs           compile() payload + Vm (step/run/reset/state)
    wasm.rs         wasm-bindgen boundary (wasm32 only)
web/                React UI
  src/
    App.tsx         state + the single synchronized highlight
    highlight.ts    provenance lookups (instr ↔ source ↔ AST)
    components/     Source (CodeMirror), Asm, Ast, Cpu, Controls
compiler-visualizer-design.md   the original design doc
```

## Scoping decisions (v1)

- No functions yet — `if`/`while`/arithmetic ships the synchronized-highlight
  demo without dragging in a calling convention. Functions are v1.1.
- ~20 RV32IM instructions; RISC-V because it's real, respected, and encodable
  in a weekend (fixed-width, four formats). x86-64 was never on the table.
- Expressions evaluate on a 7-deep register stack (t0–t6); deeper nesting is a
  friendly compile error rather than a spill engine.

## Roadmap

- [ ] Functions: calling convention, stack frames, `jal`/`jalr` — recursive
      factorial as the demo
- [ ] Breakpoints (click an assembly row to set)
- [ ] Memory panel showing the raw data section bytes
- [ ] Run-backwards (deltas are invertible by construction)
- [ ] Demo GIF in this README
