# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

First public release.

### Added

- Deobfuscation of javascript-obfuscator / obfuscator.io output: string arrays with their
  rotators, base64 and RC4 decoders and wrappers, control-flow proxy objects, `switch`
  flattening, self-defending and debug-protection code, and constant folding. Decoders run in a
  sandboxed, step-budgeted interpreter; the input is never executed.
- A safe lane (the default) that decodes only what it can prove from the program text, and an
  unsafe lane (`--unsafe`) that assumes built-ins are not patched and adds heuristic passes.
- Unminify and untranspile passes, including TypeScript, Babel, SWC and esbuild helpers.
- Unpacking of webpack 4/5, browserify, esbuild and Bun, SystemJS, AMD, UMD, Parcel, Metro,
  Turbopack and Closure bundles, with opt-in splitting of Rollup and Vite scope-hoisted bundles
  and unpacking of several input files together.
- Identifier renaming (`mangle`, `simple`, `unique`) and JSX reconstruction.
- The `untwist` command-line tool, the `untwist` library crate, and the `untwist`
  WebAssembly package used by the playground at https://untwist.dev.
