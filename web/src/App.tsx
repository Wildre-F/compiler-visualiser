// The visualizer: compile on edit, step/run the VM, and keep one shared
// highlight synchronized across source, AST, assembly/bytes, and CPU panels.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Vm } from 'engine'
import { compile } from './engine-api'
import type { CompileErr, Compilation, Highlight, Span, TracedDelta } from './types'
import { emptyHighlight, fromInstr, fromNode, fromSourcePos } from './highlight'
import { EXAMPLES } from './examples'
import { SourcePanel } from './components/SourcePanel'
import { AstPanel } from './components/AstPanel'
import { AsmPanel } from './components/AsmPanel'
import { CpuPanel } from './components/CpuPanel'
import { Controls } from './components/Controls'

const DEFAULT_EXAMPLE = 'fibonacci'

interface CpuView {
  regs: number[]
  pc: number
  halted: boolean
  varValues: number[]
}

const FRESH_CPU = (numVars: number): CpuView => ({
  regs: new Array(32).fill(0),
  pc: 0,
  halted: false,
  varValues: new Array(numVars).fill(0),
})

export default function App() {
  const [exampleName, setExampleName] = useState(DEFAULT_EXAMPLE)
  const [source, setSource] = useState(EXAMPLES[DEFAULT_EXAMPLE])
  const [compilation, setCompilation] = useState<Compilation | null>(null)
  const [error, setError] = useState<CompileErr | null>(null)

  const vmRef = useRef<Vm | null>(null)
  const [cpu, setCpu] = useState<CpuView>(FRESH_CPU(0))
  const [lastDelta, setLastDelta] = useState<TracedDelta | null>(null)
  const [stepCount, setStepCount] = useState(0)
  const [output, setOutput] = useState('')
  const [fault, setFault] = useState<string | null>(null)

  const [running, setRunning] = useState(false)
  const [speed, setSpeed] = useState(8)
  const [hover, setHover] = useState<Highlight | null>(null)

  // ---- compile on edit (debounced) ------------------------------------

  useEffect(() => {
    const t = setTimeout(() => {
      try {
        const c = compile(source)
        setCompilation(c)
        setError(null)
        vmRef.current = new Vm(source)
        setCpu(FRESH_CPU(c.vars.length))
        setLastDelta(null)
        setOutput('')
        setFault(null)
        setRunning(false)
        setHover(null)
      } catch (e) {
        setError(e as CompileErr)
        setRunning(false)
      }
    }, 250)
    return () => clearTimeout(t)
  }, [source])

  // ---- execution -------------------------------------------------------

  const applyDelta = useCallback(
    (d: TracedDelta, c: Compilation) => {
      setCpu((prev) => {
        const next: CpuView = {
          regs: prev.regs,
          pc: d.pc_after,
          halted: d.halted,
          varValues: prev.varValues,
        }
        if (d.reg_write) {
          next.regs = [...prev.regs]
          next.regs[d.reg_write.reg] = d.reg_write.new
        }
        if (d.mem_write) {
          const idx = c.vars.findIndex((v) => v.addr === d.mem_write!.addr)
          if (idx >= 0) {
            next.varValues = [...prev.varValues]
            next.varValues[idx] = d.mem_write.new
          }
        }
        return next
      })
      if (d.output) setOutput((o) => o + d.output)
      if (d.error) setFault(d.error)
      setLastDelta(d)
      setStepCount((n) => n + 1)
    },
    [],
  )

  const doStep = useCallback(() => {
    const vm = vmRef.current
    if (!vm || !compilation || vm.halted()) return
    applyDelta(vm.step() as TracedDelta, compilation)
  }, [compilation, applyDelta])

  const doReset = useCallback(() => {
    const vm = vmRef.current
    if (!vm || !compilation) return
    vm.reset()
    setCpu(FRESH_CPU(compilation.vars.length))
    setLastDelta(null)
    setOutput('')
    setFault(null)
    setRunning(false)
  }, [compilation])

  // run loop
  useEffect(() => {
    if (!running) return
    const vm = vmRef.current
    if (!vm || !compilation) return
    // at high speeds, batch multiple steps per animation tick
    const interval = speed >= 1000 ? 16 : Math.max(16, 1000 / speed)
    const batch = speed >= 1000 ? 200 : Math.max(1, Math.round(speed / 60))
    const id = setInterval(() => {
      for (let i = 0; i < batch; i++) {
        if (vm.halted()) {
          setRunning(false)
          break
        }
        applyDelta(vm.step() as TracedDelta, compilation)
      }
    }, interval)
    return () => clearInterval(id)
  }, [running, speed, compilation, applyDelta])

  // ---- the single synchronized highlight --------------------------------

  const execHighlight = useMemo<Highlight>(() => {
    if (!compilation || !lastDelta) return emptyHighlight()
    return fromInstr(compilation, lastDelta.instr_index)
  }, [compilation, lastDelta])

  // user hover/click wins over execution highlight
  const highlight = hover ?? execHighlight

  const selectSourcePos = useCallback(
    (pos: number) => {
      if (!compilation) return
      const h = fromSourcePos(compilation, pos)
      setHover(h.span ? h : null)
    },
    [compilation],
  )

  const selectAsmRow = useCallback(
    (index: number) => {
      if (!compilation) return
      setHover(fromInstr(compilation, index))
    },
    [compilation],
  )

  const selectAstNode = useCallback(
    (id: number, span: Span) => {
      if (!compilation) return
      setHover(fromNode(compilation, id, span))
    },
    [compilation],
  )

  const pcIndex =
    compilation && !cpu.halted && cpu.pc / 4 < compilation.instrs.length
      ? cpu.pc / 4
      : null

  const canStep = !!compilation && !error && !cpu.halted

  return (
    <div className="app">
      <header className="topbar">
        <h1>
          Compiler Visualizer <span className="subtitle">source → RISC-V → execution</span>
        </h1>
        <select
          className="example-picker"
          value={exampleName}
          onChange={(e) => {
            setExampleName(e.target.value)
            setSource(EXAMPLES[e.target.value])
          }}
        >
          {Object.keys(EXAMPLES).map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <Controls
          running={running}
          canStep={canStep}
          speed={speed}
          onStep={doStep}
          onRunPause={() => setRunning((r) => !r)}
          onReset={doReset}
          onSpeed={setSpeed}
        />
      </header>

      <main className="panels" onMouseLeave={() => setHover(null)}>
        <section className="panel panel-source">
          <div className="panel-title">source</div>
          <SourcePanel
            initialSource={EXAMPLES[exampleName]}
            tokens={compilation?.tokens ?? []}
            highlight={highlight}
            error={error}
            onChange={setSource}
            onSelectPos={selectSourcePos}
          />
          {error && (
            <div className="compile-error">⚠ {error.message}</div>
          )}
          <div className="panel-title">output</div>
          <pre className="console">{output || <span className="console-empty">- run the program -</span>}</pre>
        </section>

        <section className="panel panel-asm">
          <div className="panel-title">assembly &amp; machine code</div>
          {compilation && (
            <AsmPanel
              instrs={compilation.instrs}
              highlight={highlight}
              pcIndex={pcIndex}
              onSelectRow={selectAsmRow}
            />
          )}
        </section>

        <section className="panel panel-right">
          <div className="panel-title">cpu</div>
          {compilation && (
            <CpuPanel
              regs={cpu.regs}
              pc={cpu.pc}
              halted={cpu.halted}
              faulted={fault}
              vars={compilation.vars}
              varValues={cpu.varValues}
              lastDelta={lastDelta}
              stepCount={stepCount}
            />
          )}
          <div className="panel-title">ast</div>
          {compilation && (
            <AstPanel ast={compilation.ast} highlight={highlight} onSelectNode={selectAstNode} />
          )}
        </section>
      </main>
    </div>
  )
}
