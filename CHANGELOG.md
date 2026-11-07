# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Top-level calls to a small local function that updates top-level state are folded in the
  default lane when every argument is a literal: `console[test(2)](test(0), test(1), counter)`
  becomes `console.log("hello", "world", 3)`, and the function and its state are removed. The
  calls are run in evaluation order, and the fold is refused when anything else can see the
  function or the state (another function, a value use, the global object, code text).

- A javascript-obfuscator version matrix (`fixtures/obfuscator-versions`, run with
  `cargo xtask obf-options --dir fixtures/obfuscator-versions`): 723 cells over twenty releases
  from 0.10.2 to 5.9.1, string array options, top-level/IIFE/webpack 4 containers and uglify-js or
  terser after obfuscation. Its `EXPECT` records today's results, most of them misses on releases
  before 2.19.

- The version matrix files a cell whose plain string array is refused because code may reach it
  by name through the global object as that refusal (`legacy-array-reached`), not as a missed
  string array.

### Fixed

- String arrays, decoders and wrappers declared with `let` or `const` (javascript-obfuscator 0.22
  and later reuse the program's declaration kind) are found like `var` ones; a lexical array keeps
  a lexical binding when it is lifted. Machinery bound by `var`, `let` or `const` that may be called
  before its declaration runs, or a `const` that code assigns to, is left undecoded, since both
  throw.

- String arrays from old javascript-obfuscator builds (a plain `var a = [...]` with a counting
  rotator and a `var b = function (t, x) { ... a[t -= 0] ... }` decoder) were not found, and the
  run said `no string array found`. They are now decoded when the array is only read by its
  decoder; the default lane also needs the usual global-reach and patched-built-in checks to pass.
- `--unsafe` no longer refuses a top-level string array or decoder just because some string in
  the program spells its name; it refuses only when the name is reached through the global object
  or literal code text. Old builds with one-letter machinery names (`a`, `b`) now decode there,
  including the lazy `atob` set-up their base64 decoders install.
- Old-builder string arrays inside a webpack module or function, with a self-defending cookie
  rotator, rc4 decoders and wrappers in `for` heads or called in place, were not found. They
  now decode under `--unsafe`; the default lane decodes them when no rotation has to be applied.

## [0.3.0] - 2026-11-07

### Added

- `--wasm-check off|on|auto` (and Wasm check in the playground settings) decides whether the
  output is scanned for embedded wasm modules. The default is now `off`, which only notes when the
  output may hold one; `auto` extracts modules that have imports or exports, so feature-detect
  probes passed to `WebAssembly.validate` are left alone; `on` extracts every module it finds.
  Supplied and fetched modules work in every mode.
- Emscripten glue is named when its wasm module has no name section: an export wrapper
  (`_malloc = Module._malloc = exports.f`) takes the public key's name when the export is a
  function, the binding of the memory and table exports becomes `wasmMemory` and `wasmTable`,
  `Module.asm` counts as the exports holder, and the object handed to the instance as imports
  becomes `wasmImports`. Import functions whose minified names the module does not confirm stay
  unnamed.
- The names wasm glue gets in a webpack bundle's unpacked modules are also applied to the
  combined `deobfuscated.js`, each inside its own module function. If that copy does not hold the
  same bindings, a warning says so and it keeps the original names.
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

- `dead-code` now also removes branches whose condition is a literal constant: `if (false)`, `if (1)`, `while (null)`, and the `?:`, `&&` and `||` forms of the same. It keeps `var` declarations of the removed code (without initialisers), leaves a dead branch with a function declaration in sloppy mode, and never touches a labelled statement or folds a member reference out from under a call, `typeof` or `delete`.
- Terser-minified stateful decoders (base64 and rc4) decode in the safe lane: `void 0 === D.k` (either
  side, strict or loose) counts as the init gate's undefined test when `void` takes a literal, and the
  reset and temporal dead zone checks resolve the decoder's locals by scope, so a helper reusing short
  names such as `n`, `f` or `a` no longer refuses the unit.
- Rounds that add or remove scopes and bindings patch the scope tables instead of rebuilding
  them, and references left on a removed binding resolve to the outer one: game.js runs about
  3 % fewer instructions with the same output.
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
- A decode unit whose string array, decoders, rotator and call arguments are unchanged since
  an earlier round reuses that round's decoded values instead of running again, so a high-preset
  unit that is decoded and then kept as written no longer decodes twice: about 30 % fewer decode
  steps on the high preset.
- The native rc4 decoder loops also run when unminify's prepare passes have already split the
  decoder's comma sequences into statements, which is how the pipeline sees them: the high
  preset needs about 84 % fewer instructions (7.0 s to 1.2 s for its ten samples), with the
  same output.
- oxc 0.157: output is unchanged; parsing and printing take about 12 % fewer instructions, and
  game.js about 3 % fewer overall.

### Fixed

- The default lane no longer replaces a decoder argument that reads a top-level script `var`,
  function or `let` by its literal when code that may hold the global object (`globalThis.k`,
  `window.k`, top-level `this.k`, `Reflect.set`, `Object.assign`, computed keys) or code from text
  may write that name; `var k; k = 1; globalThis.k = 2; d(k)` decoded with `1`. When such code may
  only observe the name, the declaration is kept. Modules and function-local bindings still decode.
- The default lane decodes a global string array whose function or decoder name is also spelled
  by a string in the program, such as webpack 4's `'a'` getter key under mangled names; whether
  code may reach the machinery by that name is now judged where the global object is reached,
  instead of refusing on any spelling. A spelling under a direct eval still refuses.
- The default lane no longer refuses an obfuscated webpack 4 or webpack 5 bundle because of its
  bootstrap: a module's `exports` and `module` are proven to be objects the bootstrap created, and
  an AMD factory that ignores `require` no longer leaves the bootstrap unproven. A setter on the
  module cache or a read of `require.c` still refuses.
- A header-only WebAssembly feature-detect (`new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])`)
  is no longer taken for an embedded module, and output holding a real one is parsed once, not
  twice, to extract it and name its glue (game.js about 5 % faster).
- The default lane no longer inlines a variable passed to a decoder from an assignment that a write
  in the same statement has replaced: `f(d(k), [k = 2][0], d(k))` printed the wrong string. A literal
  assigned earlier in the same sequence, call or block (`(k = 2, d(k))`, `w(1, (k = 5) - 2, d(k))`,
  as terser prints them) is now inlined where no other write can run between.
- A decoder alias declared after statements that run no code (`void 0;`, `{}`, `;`) is no longer
  refused as possibly read before its declaration; such statements cannot call the hoisted
  functions that read it.
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
- The default lane decodes beside a `set` call on a local instance of a plain class (`v.set(x, y, z)` on
  `const v = new Vector3()`), which was taken for `Reflect.set` and refused, and beside a key write on
  `Object.assign({}, x)`, which was not taken for a fresh object.
- The default lane removes the decoder machinery beside `set`, `get`, `has`, `delete` and `add` calls on a
  fresh `Map`, `WeakMap`, `Set` or `WeakSet`, which were taken to observe the names of the global object
  passed to them.
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
- The default lane reads a control-flow storage object's members as the text they build, so the global-object
  getter `Function(s.Ncuch(s.IvYXD, d(1)))` and the forwarded `s.PwbRa(Function, text)` that control-flow
  flattening leaves are judged by their text instead of refusing a stateful unit.

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
