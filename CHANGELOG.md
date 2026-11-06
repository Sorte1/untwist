# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- wasm glue in a webpack bundle is named across modules: the functions an async or sync wasm
  module imports are renamed in the modules that export them, and the bindings that read its
  exports in the modules that import it, after the same check that every key and read exists in
  the supplied module. Webpack 5 bundles using `syncWebAssembly` now unpack.
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

- The default lane keeps the decoder machinery less often beside code that may hold the global
  object: calls to `JSON.parse`, `Math`, `Number`, `parseInt`, `Promise.all` and other intrinsics
  that neither print nor enumerate, and calls to `require` or to a required `path` or `url`, no
  longer count as observing its names. The last rests on a stated premise, see the README.
- The evaluator runs the obfuscator.io base64 and rc4 string-decoder loops natively, with the same step
  accounting and the same output; decoding costs less memory and time on inputs that use them.
- Unminify rounds after the first skip unchanged statements: about 6 % faster on large inputs.
- Unminify finishes the body an `if` rewrite builds in the same round, and `!` inverts nested
  comparisons at once: game.js needs 3 unminify rounds instead of 9, about 8 % fewer instructions.
- The playground worker keeps the decoded strings in one buffer while it runs, so the 10 MB sample peaks about 26 MiB lower (190 MiB instead of 216 MiB).
- The default lane's decode step budget grows with the distinct decoder calls a unit decodes
  (50 M plus 500 k a call, at most the 1 B `--unsafe` uses), so the high preset's rc4 strings
  no longer run out of steps.

### Fixed

- javascript-obfuscator output passed through terser decodes: a decoder that indexes the array call
  directly and a string array function that rewrites itself inside its return are now recognised.
  Such samples kept their string array.
- A decoder wrapper call whose arguments assign locals, as terser emits after a high preset
  (`w(1, (t = 5) - 2, t)`), is inlined with the assignments kept in order, in both lanes.
- The default lane decodes an ES module whose imports load only `node:path` or `node:url`; such
  an import runs no program code and touches no built-in, so where it sits beside the rotator no
  longer matters. Any other import still keeps the string array.
- An object literal before the rotator whose values are literals or hoisted functions
  (`var o = { k: f }`) is read as running no code, as the debug protection's holder is.
- Empty or inert blocks and expression statements that run no code (`{}`, `void 0;`) before
  the rotator no longer stop a string array from decoding.
- A string array whose array function, decoders and rotator all sit in one function body,
  such as a whole program wrapped in an IIFE, now decodes in both lanes.
- A rotator that calls the string array function by name once, as its first statement, and
  a rotator loop whose body is a bare `try` (both as terser prints them) are now trusted.
- Decoder aliases read inside the rotator, held in a one-property object, returned by an
  empty IIFE or wrapped in `(0, alias)` are inlined in both lanes, so their string arrays
  decode instead of being kept whole.
- The default lane decodes beside a getter or method of an object literal that only its own calls reach, which
  was taken for a hold on the global object through `this`.
- The default lane decodes javascript-obfuscator's domain lock: its scan of the global object's
  keys is proven unable to pick a name of the decoder machinery.
- The default lane reads the keys of an object built by keyed assignments and then aliased
  (javascript-obfuscator's transformObjectKeys) in a rest or spread, as it does for an object literal;
  such a rest was refused as one that may read any key. Its getters and methods, including the
  functions assigned to it, count as bound to it like an object literal's.
- The default lane decodes beside a fresh `new Map()` or `new WeakMap()` that is read and written only
  through its own `get` and `set`; those calls were taken for `Reflect.get` and `Reflect.set` and refused.
- A decoder wrapper whose binding is written again, or a `var` wrapper called before its
  declaration runs, is no longer inlined; both printed the wrong value in the default lane.
- An alias of a decoder wrapper (`var k = wrapper`), and wrapper calls inside the rotator,
  are resolved, so string arrays reached only through such wrappers decode in both lanes.
- The default lane now removes base64- and RC4-encoded string arrays and their decoders where it
  already removed plain ones. A decoder's own cache no longer counts as code that may hold the
  global object.
- The default lane no longer refuses a decode because of an object rest or spread, or a Symbol-valued
  computed key, on an object created in the same unit; only what such a key can read is judged.
- The default lane decodes a string array when a key or code text is built from decoded strings, such as
  `String[d(1)](...)` or `Function(d(1) + d(2))`; these were refused as a possible patch of a built-in.
- A self-defending check is removed when a cleanup left bare `var x;` declarations inside its call
  controller. It survived, and once the output was reformatted its `(((.+)+)+)+$` test hung.
- The default lane no longer overflows its stack on a variable initialised from a read of
  itself (`var a = b[a]`) or a sealed method that calls itself in an `if` test; such inputs,
  seen in dead-code-injected obfuscator output, used to stop the worker and pass through
  unchanged.
- The default lane reads `cache[d(1)](key)` on a fresh `new Map()` by the method name `d(1)` decodes to, so a
  memoize cache whose method names were moved into the string array no longer refuses the unit.

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
