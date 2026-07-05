import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  // Relative base so the built app works when served from a GitHub Pages
  // project subpath (e.g. /compiler-visualiser/) as well as from root.
  base: './',
  plugins: [react()],
  // The engine is a wasm-pack output linked via file: — prebundling it would
  // break the `new URL('engine_bg.wasm', import.meta.url)` resolution.
  optimizeDeps: { exclude: ['engine'] },
  server: {
    fs: { allow: ['..'] },
  },
})
