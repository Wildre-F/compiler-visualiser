// TypeScript mirror of the engine's serialized payloads (see engine/src/vm.rs).

export interface Span {
  start: number
  end: number
}

export interface Token {
  kind: string
  text: string
  span: Span
}

// AST - statements and expressions are tagged unions via `type`.
export interface AstNodeBase {
  id: number
  span: Span
}

export type Stmt = AstNodeBase &
  (
    | { type: 'let'; name: string; init: Expr }
    | { type: 'assign'; name: string; value: Expr }
    | { type: 'if'; cond: Expr; then_body: Stmt[]; else_body: Stmt[] | null }
    | { type: 'while'; cond: Expr; body: Stmt[] }
    | { type: 'print'; value: Expr }
  )

export type Expr = AstNodeBase &
  (
    | { type: 'int'; value: number }
    | { type: 'var'; name: string }
    | { type: 'unary'; op: string; operand: Expr }
    | { type: 'binary'; op: string; lhs: Expr; rhs: Expr }
  )

export interface AsmRow {
  addr: number
  word: number
  asm: string
  node: number | null
  span: Span | null
}

export interface VarSlot {
  name: string
  addr: number
}

export interface Compilation {
  tokens: Token[]
  ast: Stmt[]
  instrs: AsmRow[]
  vars: VarSlot[]
  data_base: number
}

export interface CompileErr {
  message: string
  span: Span | null
}

export interface RegWrite {
  reg: number
  old: number
  new: number
}

export interface MemWrite {
  addr: number
  old: number
  new: number
}

export interface TracedDelta {
  pc_before: number
  pc_after: number
  instr_index: number
  reg_write: RegWrite | null
  mem_write: MemWrite | null
  output: string | null
  halted: boolean
  error: string | null
  span: Span | null
}

export interface VmState {
  regs: number[]
  pc: number
  halted: boolean
  var_values: number[]
}

/** The single "current highlight" concept shared by every panel. */
export interface Highlight {
  /** Source character range to light up. */
  span: Span | null
  /** AST node ids to light up. */
  nodes: Set<number>
  /** Instruction indices (asm rows + byte groups) to light up. */
  instrs: Set<number>
}

export const RISCV_REG_NAMES = [
  'zero', 'ra', 'sp', 'gp', 'tp', 't0', 't1', 't2', 's0', 's1', 'a0', 'a1',
  'a2', 'a3', 'a4', 'a5', 'a6', 'a7', 's2', 's3', 's4', 's5', 's6', 's7',
  's8', 's9', 's10', 's11', 't3', 't4', 't5', 't6',
]
