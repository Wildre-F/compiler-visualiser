// The assembly + machine-code listing: address | raw bytes | mnemonic.
//
// The bytes column shows the actual little-endian bytes in memory — the same
// bytes the CPU fetches and decodes each cycle.

import { useEffect, useRef } from 'react'
import type { AsmRow, Highlight } from '../types'

interface Props {
  instrs: AsmRow[]
  highlight: Highlight
  /** Row the PC currently points at (distinct from hover highlight). */
  pcIndex: number | null
  onSelectRow: (index: number) => void
}

function hexBytes(word: number): string {
  // little-endian byte order, the way the bytes actually sit in memory
  const b = [word & 0xff, (word >> 8) & 0xff, (word >> 16) & 0xff, (word >> 24) & 0xff]
  return b.map((x) => x.toString(16).padStart(2, '0')).join(' ')
}

export function AsmPanel({ instrs, highlight, pcIndex, onSelectRow }: Props) {
  const bodyRef = useRef<HTMLDivElement>(null)

  // keep the PC row scrolled into view while stepping
  useEffect(() => {
    if (pcIndex === null) return
    const el = bodyRef.current?.querySelector(`[data-row="${pcIndex}"]`)
    el?.scrollIntoView({ block: 'nearest' })
  }, [pcIndex])

  return (
    <div className="asm-listing" ref={bodyRef}>
      <div className="asm-head">
        <span className="asm-col-pc" />
        <span className="asm-col-addr">addr</span>
        <span className="asm-col-bytes">machine code</span>
        <span className="asm-col-asm">assembly</span>
      </div>
      {instrs.map((row, i) => {
        const isPc = pcIndex === i
        const isHl = highlight.instrs.has(i)
        return (
          <div
            key={row.addr}
            data-row={i}
            className={`asm-row${isHl ? ' hl-active' : ''}${isPc ? ' asm-pc' : ''}`}
            onClick={() => onSelectRow(i)}
          >
            <span className="asm-col-pc">{isPc ? '▶' : ''}</span>
            <span className="asm-col-addr">{row.addr.toString(16).padStart(4, '0')}</span>
            <span className="asm-col-bytes">{hexBytes(row.word)}</span>
            <span className="asm-col-asm">{row.asm}</span>
          </div>
        )
      })}
    </div>
  )
}
