<div align="center">

# ⚙️ Compiler Visualiser

**Source to silicon, step by step, all in your browser.**

Write a small program, watch it compile through every stage, then step a simulated RV32IM CPU
through the actual machine-code bytes one fetch-decode-execute cycle at a time.

![Rust](https://img.shields.io/badge/Rust-WebAssembly-5c5fd4?style=flat-square&logoColor=white)
![React](https://img.shields.io/badge/React-TypeScript-1d9e75?style=flat-square&logoColor=white)
![RISC-V](https://img.shields.io/badge/Target-RV32IM-7f77dd?style=flat-square&logoColor=white)
![Open Source](https://img.shields.io/badge/Open-Source-ff69b4?style=flat-square&logoColor=white)

[![Live Demo](https://img.shields.io/badge/▶%20Live%20Demo-1d9e75?style=for-the-badge&logoColor=white)](https://wildre-f.github.io/compiler-visualiser/)

<img src="https://skillicons.dev/icons?i=rust,wasm,react,ts,vite" />

</div>

---

##  Overview

Click any line of source and the matching AST nodes, assembly rows and machine-code bytes light up.
Step the CPU and the highlight runs the other way, from the executing instruction back to the source
that produced it.

```
   Source ──lex──▶ Tokens ──parse──▶ AST ──codegen──▶ RISC-V asm ──assemble──▶ bytes
      ▲                                                                          │
      └────────────────── synchronised highlight ◀────── CPU executes ◀─────────┘
```

##  Why it's interesting

- **Provenance end to end.** Every token, AST node, instruction and byte carries a tag pointing back
  to the source span it came from. That single discipline is what makes the synchronised highlight
  possible.
- **The bytes are real.** Instructions assemble to genuine RV32IM encodings (verified against
  known-good words). The simulator decodes the actual bytes in memory every cycle, so the
  machine-code panel is not decoration, it is what executes.
- **A clean engine/UI boundary.** The engine is pure Rust compiled to WebAssembly: `compile()`
  returns every artifact in one payload, and `vm.step()` returns a small state delta per cycle. React
  renders state and forwards clicks; it computes nothing.

##  Tech stack

| Layer  | Tech                                     |
|--------|------------------------------------------|
| Engine | Rust to WebAssembly (wasm-bindgen)       |
| UI     | React + TypeScript + Vite + CodeMirror 6 |
| Deploy | Fully static, no backend                 |

##  The language

A tiny C-like language: integers, `let`, assignment, arithmetic (`+ - * / %`), comparisons,
`if`/`else`, `while` and `print()`. Enough for fibonacci, collatz and trial-division primes (all
included as examples). `print(x)` compiles to a print-int ecall; programs end with an exit ecall.
Division by zero returns -1, because that is what the RISC-V spec says real hardware does.

##  Run it locally

```bash
git clone https://github.com/Wildre-F/compiler-visualiser.git
cd compiler-visualiser

# 1. build the engine (needs Rust + wasm-pack)
cd engine && wasm-pack build --target web

# 2. run the UI
cd ../web && npm install && npm run dev
```

Then open http://localhost:5173.

```bash
# engine test suite (33 tests, incl. fibonacci end-to-end)
cd engine && cargo test
```

##  Project layout

```
engine/             pure Rust core (cdylib + rlib)
  src/
    span.rs         Span + CompileError, the provenance backbone
    lexer.rs        text to spanned tokens
    ast.rs          AST node types (id + span on every node)
    parser.rs       recursive descent to AST
    ir.rs           the RV32IM instruction subset
    codegen.rs      AST to instructions (gp-relative variable slots,
                    register-stack expressions, label fixups)
    assembler.rs    instruction to/from 32-bit word (encode + decode)
    cpu.rs          fetch/decode/execute core, per-cycle deltas
    vm.rs           compile() payload + Vm (step/run/reset/state)
    wasm.rs         wasm-bindgen boundary (wasm32 only)
web/                React UI
  src/
    App.tsx         state + the single synchronised highlight
    highlight.ts    provenance lookups (instr <-> source <-> AST)
    components/     Source (CodeMirror), Asm, Ast, Cpu, Controls
```

##  Design decisions (v1)

- No functions yet: `if`/`while`/arithmetic ships the synchronised-highlight demo without dragging in
  a calling convention. Functions are v1.1.
- ~20 RV32IM instructions; RISC-V because it is real, respected and encodable in a weekend
  (fixed-width, four formats).
- Expressions evaluate on a 7-deep register stack (t0-t6); deeper nesting is a friendly compile error
  rather than a spill engine.

##  Roadmap

- [ ] Functions: calling convention, stack frames, `jal`/`jalr`, recursive factorial as the demo
- [ ] Breakpoints (click an assembly row to set)
- [ ] Memory panel showing the raw data-section bytes
- [ ] Run-backwards (deltas are invertible by construction)

<br/>

<div align="center">
  <img src="https://readme-typing-svg.demolab.com?font=Fira+Code&size=13&pause=1000&color=7f77dd&center=true&vCenter=true&width=560&lines=From+source+text+to+running+silicon." alt="Footer" />
</div>
