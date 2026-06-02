import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  // The engine is a wasm-pack output linked via file: — prebundling it would
  // break the `new URL('engine_bg.wasm', import.meta.url)` resolution.
  optimizeDeps: { exclude: ['engine'] },
  server: {
    fs: { allow: ['..'] },
  },
})
