# Compiler Visualizer — Design Document

An interactive, browser-based tool that shows how high-level code is broken down
into assembly and machine code, then executed by a simulated CPU — with every
stage highlighted in sync, step by step.

---

## 1. Goal

Build a single-screen tool where a user can:

1. Write a small program in a high-level language.
2. See it transformed through every compilation stage: **source → tokens → AST →
   assembly → machine code (raw bytes)**.
3. Watch a simulated CPU execute those bytes one instruction at a time, with
   registers and memory updating live.
4. See all of this **synchronized**: clicking a line of source highlights the
   corresponding assembly, the corresponding bytes, and the CPU activity.

The synchronized, animated, end-to-end view is the product. Compiler Explorer
(godbolt.org) already nails source → assembly; the differentiator here is the
**execution simulation** plus the **unified, synchronized narrative**.

---

## 2. Tech stack

| Layer  | Technology                              | Why |
|--------|-----------------------------------------|-----|
| Engine | Rust, compiled to WebAssembly           | Compilers and VMs are a near-perfect fit for Rust's enums and pattern matching. WASM means it runs entirely in the browser. Strong portfolio signal. |
| UI     | TypeScript + React                      | All the visual value lives here; React renders engine state and forwards user actions. |
| Deploy | Static site (no backend)                | Trivial to host and demo. The whole app is a static bundle. |

The boundary between engine and UI is deliberately clean, so the engine could be
reimplemented in another language without touching the UI — and vice versa.

---

## 3. Core principle: provenance

Every artifact the engine produces — every token, AST node, assembly instruction,
and individual byte of machine code — carries a tag pointing back to the **source
span** it came from.

- A token knows: "I'm characters 12–15 on line 3."
- The AST node built from it inherits that span.
- The instruction generated from that node inherits it.
- The bytes encoding that instruction inherit it too.

This single discipline is what makes the synchronized highlight possible. **Build
it in from day one** — retrofitting provenance later is painful.

---

## 4. Architecture overview

```
                        Source code
                             |
                             v
   +---------------------------------------------------------+
   |        Rust engine — compiled to WebAssembly            |
   |                                                         |
   |   +------------------+        +----------------------+  |
   |   |    Compiler      |  bytes |    CPU simulator     |  |
   |   |  Lexer           | -----> |  Registers (x0-x31)  |  |
   |   |  Parser -> AST   |        |  Memory              |  |
   |   |  Codegen -> asm  |        |  Program counter     |  |
   |   |  Assembler->bytes|        |  fetch/decode/execute|  |
   |   +------------------+        +----------------------+  |
   +---------------------------------------------------------+
        |  compile() -> all artifacts (once)
        |  step()    -> state delta (per cycle)
        v
   +---------------------------------------------------------+
   |          React UI — renders synchronized state          |
   |   Source | AST | Assembly | Hex bytes | CPU state       |
   +---------------------------------------------------------+
```

---

## 5. Data flow

### 5.1 Compile time (runs once per edit)

1. The UI passes the source string into the engine's `compile()` function.
2. **Lexer** turns raw text into tokens, each tagged with its character range.
3. **Parser** folds tokens into an AST; each node remembers its source span.
4. **Codegen** walks the tree and emits RISC-V instructions, each tagged with the
   AST node it came from.
5. **Assembler** encodes each instruction into its actual 32-bit machine word —
   the real bytes — preserving the provenance link.
6. `compile()` returns *all* artifacts at once (tokens, AST, instructions, bytes,
   provenance links) in one payload. React stores it and paints the panels.

This is the cheap "do it once" half.

### 5.2 Run time (runs in a loop as the user steps)

The engine holds a `CpuState` struct: registers, a memory block, the program
counter. Each `step()` call does one **fetch–decode–execute** cycle:

1. **Fetch** — read 4 bytes from memory at the program counter.
2. **Decode** — parse those bits back into an instruction. *(Decode from the
   actual bytes, not a cached instruction list — the whole point is to show that
   the machine code genuinely drives execution.)*
3. **Execute** — apply the effect: write a register, move the PC, touch memory.
4. **Return a delta** — e.g. "PC 0x10 → 0x14, register x5 = 7, instruction traces
   to source span 4."

The boundary stays lean: a small delta per cycle, never the whole memory array.
Running thousands of instructions stays smooth.

### 5.3 Synchronized highlight (the showpiece)

When the PC lands on instruction N, React looks up N's provenance and lights up
the matching source line, AST node, assembly row, and hex bytes simultaneously,
while the CPU panel animates the register that just changed. Clicking a source
line runs the same lookup in reverse to highlight everything downstream.

---

## 6. Engine API (the WASM boundary)

Keep the boundary **chunky, not chatty**:

| Call            | Returns                                             | Frequency |
|-----------------|-----------------------------------------------------|-----------|
| `compile(src)`  | All artifacts + provenance links                    | Once per edit |
| `vm.step()`     | State delta (PC change, register/memory change, span)| Per cycle |
| `vm.run()`      | Runs to completion / breakpoint                     | On demand |
| `vm.reset()`    | Resets CpuState                                     | On demand |
| `vm.state()`    | Full current CpuState (for initial paint)           | Rarely |

Design notes:

- **The engine is pure.** Same input, same output, no hidden state except the
  explicit `CpuState`, which can be reset and inspected. This makes it trivially
  testable — a clean thing to show on GitHub.
- **React is a dumb renderer.** It never computes anything about compilation or
  execution; it only displays state the engine hands it and forwards clicks back.
  Holding this boundary is what keeps the project from turning into spaghetti.

---

## 7. UI panels

| Panel       | Shows                                              |
|-------------|----------------------------------------------------|
| Source      | The editable program, with span highlighting       |
| AST         | The parsed tree as a collapsible view               |
| Assembly    | The RISC-V instruction list                         |
| Hex bytes   | The raw machine code                                |
| CPU state   | Register file, memory view, PC indicator, stack     |
| Controls    | Step, Run, Reset, speed slider, (later) breakpoints |

There is a single "current highlight" concept driven either by the executing
instruction during stepping, or by the user hovering/clicking a panel.

---

## 8. Scoping decisions (what keeps it finishable)

The biggest risk is over-ambition. Deliberate simplifications:

- **Source language:** a tiny C-like subset — ints, arithmetic, `if`/`while`,
  functions. Enough to write `factorial` and `fibonacci`; not enough to need a
  real type system. (A small custom language is also fine.)
- **Target ISA:** **RISC-V** (a clean subset). It's real, respected, fixed-width,
  and beautifully simple to encode. "Compiles to RISC-V" sounds far more
  impressive than a toy ISA, and it stays tractable. This single choice is what
  keeps the project shippable — *do not* attempt real x86-64.
- **CPU model:** byte-addressable memory, the RISC-V register file (x0 hardwired
  to zero), a program counter, and a step-able fetch/decode/execute loop.

---

## 9. Minimum impressive version (ship this first)

- Fixed/editable input in the small language.
- ~15 RISC-V instructions supported.
- Register + memory panels.
- Step / Run / Reset controls.
- Source ↔ assembly ↔ bytes ↔ CPU synchronized highlighting.

Ship that, record the demo GIF, post it — *then* expand (more instructions,
breakpoints, an AST animation, a v2 rewrite blog post, etc.).

---

## 10. What makes it portfolio-worthy

- Demonstrates understanding of systems **all the way down**: language → IR →
  machine code → CPU.
- Visual and demoable — a GIF stops the scroll on LinkedIn; the repo earns stars
  because it's a genuine learning tool.
- The Rust + WASM engine is a real flex and a natural fit for the problem.
- The pure, testable engine and disciplined engine/UI boundary are themselves
  signals of good engineering judgment.
