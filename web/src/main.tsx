import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import init from 'engine'
import App from './App'
import './styles.css'

// Load the WASM engine once, before the first render.
init().then(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
})
