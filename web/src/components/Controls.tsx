// Step / Run / Reset / speed - the execution controls.

interface Props {
  running: boolean
  canStep: boolean
  speed: number // steps per second
  onStep: () => void
  onRunPause: () => void
  onReset: () => void
  onSpeed: (sps: number) => void
}

export function Controls({ running, canStep, speed, onStep, onRunPause, onReset, onSpeed }: Props) {
  return (
    <div className="controls">
      <button className="btn" onClick={onStep} disabled={!canStep || running}>
        Step
      </button>
      <button className="btn btn-primary" onClick={onRunPause} disabled={!canStep && !running}>
        {running ? 'Pause' : 'Run'}
      </button>
      <button className="btn" onClick={onReset}>
        Reset
      </button>
      <label className="speed">
        <input
          type="range"
          min={0}
          max={100}
          value={speedToSlider(speed)}
          onChange={(e) => onSpeed(sliderToSpeed(Number(e.target.value)))}
        />
        <span>{speed >= 1000 ? 'max' : `${speed}/s`}</span>
      </label>
    </div>
  )
}

// log scale: 1/s .. 1000/s ("max")
function sliderToSpeed(v: number): number {
  const s = Math.round(Math.pow(10, (v / 100) * 3))
  return s
}

function speedToSlider(s: number): number {
  return Math.round((Math.log10(Math.max(1, s)) / 3) * 100)
}
