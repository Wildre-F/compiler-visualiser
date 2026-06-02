// The editable source panel — CodeMirror 6 with decoration-driven highlights.
//
// Three decoration layers, all rebuilt via effects dispatched from App:
//  - token classes (syntax colors come from the engine's own lexer output)
//  - the synchronized execution/hover highlight
//  - compile-error underline

import { useEffect, useRef } from 'react'
import { EditorView, Decoration, type DecorationSet, keymap } from '@codemirror/view'
import { EditorState, StateEffect, StateField } from '@codemirror/state'
import { basicSetup } from 'codemirror'
import { indentWithTab } from '@codemirror/commands'
import type { CompileErr, Highlight, Span, Token } from '../types'

interface DecoRange {
  from: number
  to: number
  cls: string
}

const setDecorations = StateEffect.define<DecoRange[]>()

const decoField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    deco = deco.map(tr.changes)
    for (const e of tr.effects) {
      if (e.is(setDecorations)) {
        deco = Decoration.set(
          e.value
            .filter((r) => r.from < r.to)
            .map((r) => Decoration.mark({ class: r.cls }).range(r.from, r.to)),
          true,
        )
      }
    }
    return deco
  },
  provide: (f) => EditorView.decorations.from(f),
})

const TOKEN_CLASS: Record<string, string> = {
  let: 'tok-kw',
  if: 'tok-kw',
  else: 'tok-kw',
  while: 'tok-kw',
  print: 'tok-kw',
  int: 'tok-num',
  ident: 'tok-ident',
}

interface Props {
  initialSource: string
  tokens: Token[]
  highlight: Highlight
  error: CompileErr | null
  onChange: (src: string) => void
  onSelectPos: (pos: number) => void
}

export function SourcePanel({ initialSource, tokens, highlight, error, onChange, onSelectPos }: Props) {
  const hostRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const callbacks = useRef({ onChange, onSelectPos })
  callbacks.current = { onChange, onSelectPos }

  // Create the editor once.
  useEffect(() => {
    const view = new EditorView({
      state: EditorState.create({
        doc: initialSource,
        extensions: [
          basicSetup,
          keymap.of([indentWithTab]),
          decoField,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) callbacks.current.onChange(u.state.doc.toString())
          }),
          EditorView.domEventHandlers({
            click: (_e, v) => {
              const pos = v.state.selection.main.head
              callbacks.current.onSelectPos(pos)
            },
          }),
          EditorView.theme({}, { dark: true }),
        ],
      }),
      parent: hostRef.current!,
    })
    viewRef.current = view
    return () => view.destroy()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // If App swaps the whole program (example selector), replace the doc.
  useEffect(() => {
    const view = viewRef.current
    if (view && view.state.doc.toString() !== initialSource) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: initialSource },
      })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialSource])

  // Rebuild decorations whenever tokens / highlight / error change.
  useEffect(() => {
    const view = viewRef.current
    if (!view) return
    const docLen = view.state.doc.length
    const clamp = (s: Span) => ({ from: Math.min(s.start, docLen), to: Math.min(s.end, docLen) })

    const ranges: DecoRange[] = []
    for (const t of tokens) {
      const cls = TOKEN_CLASS[t.kind]
      if (cls) ranges.push({ ...clamp(t.span), cls })
    }
    if (highlight.span) {
      ranges.push({ ...clamp(highlight.span), cls: 'hl-active' })
    }
    if (error?.span) {
      const r = clamp(error.span)
      // make zero-width error spans (e.g. at EOF) visible on the previous char
      if (r.from === r.to && r.from > 0) r.from -= 1
      ranges.push({ ...r, cls: 'hl-error' })
    }
    view.dispatch({ effects: setDecorations.of(ranges) })
  }, [tokens, highlight, error])

  return <div className="source-editor" ref={hostRef} />
}
