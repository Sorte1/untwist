# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Unpack writes wasm modules embedded as base64, data URIs or byte arrays (wasm-bindgen,
  Emscripten `SINGLE_FILE`, webpack inline assets) to `wasm/*.wasm` beside the output and
  points the literal at that file. Each payload is checked for the `\0asm` header and a size
  limit, decoded statically and never run; its import and export names name the JS glue when
  they match it. Remote wasm (`fetch`, `new URL`, `locateFile`, webpack's hashed
  `.module.wasm`) is listed in the report with the name to supply it under; nothing is fetched.
- `--wasm-dir DIR` supplies wasm modules the input loads from elsewhere, matched by file name
  (hashed names included), and `--wasm NAME=PATH` overrides one. `--fetch-wasm` downloads the
  rest from absolute URLs, or relative ones against `--wasm-base URL`; without it nothing is
  fetched. A supplied or fetched module that does not cover the glue's imports and exports is
  reported and names nothing.
- The playground lists extracted wasm modules with a download button each. Dropping `.wasm`
  files on the page matches them to the modules the code loads by name. Each remote module can
  be fetched after a prompt naming its URL (relative ones ask for a base first); an "Always
  fetch wasm" setting, off by default and kept in the browser only, skips the prompt.

### Changed

- Unminify rounds after the first skip unchanged statements: about 6 % faster on large inputs.
- The playground worker keeps the decoded strings in one buffer while it runs, so the 10 MB sample peaks about 26 MiB lower (190 MiB instead of 216 MiB).

### Fixed

- The default lane decodes beside a fresh `new Map()` or `new WeakMap()` that is read and written only
  through its own `get` and `set`; those calls were taken for `Reflect.get` and `Reflect.set` and refused.
- A decoder wrapper whose binding is written again, or a `var` wrapper called before its
  declaration runs, is no longer inlined; both printed the wrong value in the default lane.
- An alias of a decoder wrapper (`var k = wrapper`), and wrapper calls inside the rotator,
  are resolved, so string arrays reached only through such wrappers decode in both lanes.
- The default lane no longer refuses a decode because of an object rest or spread, or a Symbol-valued
  computed key, on an object created in the same unit; only what such a key can read is judged.
- The default lane decodes a string array when a key or code text is built from decoded strings, such as
  `String[d(1)](...)` or `Function(d(1) + d(2))`; these were refused as a possible patch of a built-in.
- A self-defending check is removed when a cleanup left bare `var x;` declarations inside its call
  controller. It survived, and once the output was reformatted its `(((.+)+)+)+$` test hung.

## [0.2.1] - 2026-11-05

### Fixed

- `--unsafe` decodes a string array beside code that may hold the global object, such as
  core-js's global lookup or Babel's spread and construct helpers, as webcrack does. Such
  bundles were left fully encoded; the default lane still refuses them.

## [0.2.0] - 2026-11-04

### Added

- `--unsafe` now decodes everything webcrack 2.16 decodes on every free javascript-obfuscator
  option, checked by an option matrix of 1,100 generated samples in CI.
- Calls to pure lookup helpers with literal arguments fold to their value, so
  `console[dec(2)](dec(0), dec(1))` prints as `console.log("hello", "worold")`.
- A whole-program value-flow analysis lets the safe lane decode string arrays whose reads
  escape into code it can prove inert.
- Decoders that reset their memo cache (newer javascript-obfuscator releases) are decoded.
- Playground: refusal explanations, pass-by-pass stepping, a decoded-strings table and linked
  selection.

### Changed

- `--unsafe` assumes built-ins are pristine even when the program patches one, as webcrack does,
  and gets a larger step budget.

### Fixed

- Regex escapes such as `\b` no longer count as spelling a decoder name.
- Decodes deferred while wrappers were still being inlined are retried instead of silently
  skipped.
- Computed `Object.keys` keys and unused hoisted variables left behind by dead-code injection.
- The playground's Steps view works in Safari under the strict content security policy.

## [0.1.0] - 2026-11-01

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
