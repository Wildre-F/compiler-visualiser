// Typed facade over the WASM engine.
//
// serde-wasm-bindgen serializes Rust HashMaps as JS Maps and structs as plain
// objects; our payloads are all structs/arrays, so a cast is sufficient.

import { compile as wasmCompile } from 'engine'
import type { Compilation } from './types'

export function compile(src: string): Compilation {
  return wasmCompile(src) as Compilation
}
