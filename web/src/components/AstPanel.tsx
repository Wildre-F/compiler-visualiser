// The AST panel - the parsed tree, collapsible, highlight-synchronized.

import { useState } from 'react'
import type { Expr, Highlight, Span, Stmt } from '../types'

interface Props {
  ast: Stmt[]
  highlight: Highlight
  onSelectNode: (id: number, span: Span) => void
}

export function AstPanel({ ast, highlight, onSelectNode }: Props) {
  return (
    <div className="ast-tree">
      {ast.map((s) => (
        <StmtNode key={s.id} stmt={s} highlight={highlight} onSelect={onSelectNode} />
      ))}
    </div>
  )
}

interface NodeRowProps {
  id: number
  span: Span
  label: string
  detail?: string
  highlight: Highlight
  onSelect: (id: number, span: Span) => void
  children?: React.ReactNode
}

function NodeRow({ id, span, label, detail, highlight, onSelect, children }: NodeRowProps) {
  const [open, setOpen] = useState(true)
  const active = highlight.nodes.has(id)
  const hasKids = children != null
  return (
    <div className="ast-node">
      <div
        className={`ast-row${active ? ' hl-active' : ''}`}
        onClick={() => onSelect(id, span)}
      >
        {hasKids ? (
          <button
            className="ast-toggle"
            onClick={(e) => {
              e.stopPropagation()
              setOpen(!open)
            }}
          >
            {open ? '▾' : '▸'}
          </button>
        ) : (
          <span className="ast-toggle-spacer" />
        )}
        <span className="ast-label">{label}</span>
        {detail && <span className="ast-detail">{detail}</span>}
      </div>
      {hasKids && open && <div className="ast-children">{children}</div>}
    </div>
  )
}

function StmtNode({ stmt, highlight, onSelect }: { stmt: Stmt; highlight: Highlight; onSelect: NodeRowProps['onSelect'] }) {
  const common = { id: stmt.id, span: stmt.span, highlight, onSelect }
  switch (stmt.type) {
    case 'let':
      return (
        <NodeRow {...common} label="let" detail={stmt.name}>
          <ExprNode expr={stmt.init} highlight={highlight} onSelect={onSelect} />
        </NodeRow>
      )
    case 'assign':
      return (
        <NodeRow {...common} label="assign" detail={stmt.name}>
          <ExprNode expr={stmt.value} highlight={highlight} onSelect={onSelect} />
        </NodeRow>
      )
    case 'print':
      return (
        <NodeRow {...common} label="print">
          <ExprNode expr={stmt.value} highlight={highlight} onSelect={onSelect} />
        </NodeRow>
      )
    case 'if':
      return (
        <NodeRow {...common} label="if">
          <ExprNode key="cond" expr={stmt.cond} highlight={highlight} onSelect={onSelect} />
          <div key="then-label" className="ast-branch">then</div>
          {stmt.then_body.map((s) => (
            <StmtNode key={s.id} stmt={s} highlight={highlight} onSelect={onSelect} />
          ))}
          {stmt.else_body && <div key="else-label" className="ast-branch">else</div>}
          {stmt.else_body?.map((s) => (
            <StmtNode key={s.id} stmt={s} highlight={highlight} onSelect={onSelect} />
          ))}
        </NodeRow>
      )
    case 'while':
      return (
        <NodeRow {...common} label="while">
          <ExprNode key="cond" expr={stmt.cond} highlight={highlight} onSelect={onSelect} />
          <div key="body-label" className="ast-branch">body</div>
          {stmt.body.map((s) => (
            <StmtNode key={s.id} stmt={s} highlight={highlight} onSelect={onSelect} />
          ))}
        </NodeRow>
      )
  }
}

function ExprNode({ expr, highlight, onSelect }: { expr: Expr; highlight: Highlight; onSelect: NodeRowProps['onSelect'] }) {
  const common = { id: expr.id, span: expr.span, highlight, onSelect }
  switch (expr.type) {
    case 'int':
      return <NodeRow {...common} label="int" detail={String(expr.value)} />
    case 'var':
      return <NodeRow {...common} label="var" detail={expr.name} />
    case 'unary':
      return (
        <NodeRow {...common} label="neg">
          <ExprNode expr={expr.operand} highlight={highlight} onSelect={onSelect} />
        </NodeRow>
      )
    case 'binary':
      return (
        <NodeRow {...common} label={expr.op}>
          <ExprNode expr={expr.lhs} highlight={highlight} onSelect={onSelect} />
          <ExprNode expr={expr.rhs} highlight={highlight} onSelect={onSelect} />
        </NodeRow>
      )
  }
}
