// The CPU state panel: PC, the register file, and variable memory slots.
// The register / memory cell touched by the last cycle flashes.

import type { TracedDelta, VarSlot } from '../types'
import { RISCV_REG_NAMES } from '../types'

interface Props {
  regs: number[]
  pc: number
  halted: boolean
  faulted: string | null
  vars: VarSlot[]
  varValues: number[]
  lastDelta: TracedDelta | null
  /** Forces the flash animation to restart every step. */
  stepCount: number
}

export function CpuPanel({ regs, pc, halted, faulted, vars, varValues, lastDelta, stepCount }: Props) {
  const changedReg = lastDelta?.reg_write?.reg ?? null
  const changedAddr = lastDelta?.mem_write?.addr ?? null

  return (
    <div className="cpu-panel">
      <div className="cpu-status">
        <span className="cpu-pc">
          PC <b>{pc.toString(16).padStart(4, '0')}</b>
        </span>
        {faulted ? (
          <span className="cpu-badge cpu-fault">fault: {faulted}</span>
        ) : halted ? (
          <span className="cpu-badge cpu-halted">halted</span>
        ) : (
          <span className="cpu-badge cpu-running">ready</span>
        )}
      </div>

      <div className="reg-grid">
        {regs.map((v, i) => (
          <div
            key={i}
            className={`reg-cell${i === changedReg ? ' flash' : ''}`}
            // remounting on each step restarts the CSS animation
            {...(i === changedReg ? { 'data-step': stepCount } : {})}
            title={`x${i}`}
          >
            <span className="reg-name">{RISCV_REG_NAMES[i]}</span>
            <span className="reg-value">{v}</span>
          </div>
        ))}
      </div>

      {vars.length > 0 && (
        <>
          <div className="panel-subtitle">variables (data section)</div>
          <div className="var-table">
            {vars.map((slot, i) => (
              <div
                key={slot.name}
                className={`var-row${slot.addr === changedAddr ? ' flash' : ''}`}
                {...(slot.addr === changedAddr ? { 'data-step': stepCount } : {})}
              >
                <span className="var-addr">{slot.addr.toString(16).padStart(4, '0')}</span>
                <span className="var-name">{slot.name}</span>
                <span className="var-value">{varValues[i] ?? 0}</span>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
