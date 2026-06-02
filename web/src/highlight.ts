// The provenance lookups behind the synchronized highlight (design doc §5.3).
//
// Every artifact carries links back to the source span / AST node it came
// from, so a highlight can start from any panel and propagate to all others.

import type { Compilation, Highlight, Span } from './types'

const EMPTY: Highlight = { span: null, nodes: new Set(), instrs: new Set() }

export function emptyHighlight(): Highlight {
  return EMPTY
}

/** Highlight driven by the executing instruction (PC landed on row `index`). */
export function fromInstr(c: Compilation, index: number): Highlight {
  const row = c.instrs[index]
  if (!row) return EMPTY
  const nodes = new Set<number>()
  if (row.node !== null) nodes.add(row.node)
  return { span: row.span, nodes, instrs: new Set([index]) }
}

/** Highlight driven by clicking/hovering an AST node. */
export function fromNode(c: Compilation, nodeId: number, span: Span): Highlight {
  const instrs = new Set<number>()
  for (let i = 0; i < c.instrs.length; i++) {
    if (c.instrs[i].node === nodeId) instrs.add(i)
  }
  return { span, nodes: new Set([nodeId]), instrs }
}

/**
 * Highlight driven by a position in the source: find the *innermost* spans
 * (smallest covering range) among instructions that contain the position,
 * then light up everything generated from them.
 */
export function fromSourcePos(c: Compilation, pos: number): Highlight {
  let best: Span | null = null
  for (const row of c.instrs) {
    const s = row.span
    if (!s || pos < s.start || pos >= s.end) continue
    if (!best || s.end - s.start < best.end - best.start) best = s
  }
  if (!best) return EMPTY

  const nodes = new Set<number>()
  const instrs = new Set<number>()
  for (let i = 0; i < c.instrs.length; i++) {
    const s = c.instrs[i].span
    if (s && s.start === best.start && s.end === best.end) {
      instrs.add(i)
      const n = c.instrs[i].node
      if (n !== null) nodes.add(n)
    }
  }
  return { span: best, nodes, instrs }
}
