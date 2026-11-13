<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
    <img src="docs/assets/logo.svg" alt="Untwist" width="270">
  </picture>
</p>

Untwist is a JavaScript deobfuscator, unminifier and unbundler written in Rust on the
[oxc](https://oxc.rs) toolchain. It reads obfuscated, minified or bundled JavaScript and gives back
readable source: string arrays decoded, control flow unflattened, minifier idioms undone,
transpiler helpers turned back into native syntax, bundles split into modules and, on request,
identifiers renamed.

It is a reimplementation of [webcrack](https://github.com/j4k0xb/webcrack) with two goals: speed,
and deobfuscating hostile input without running it. Untwist never executes the program it is
given. String decoders run inside a sandboxed, step-budgeted interpreter, and a decode is applied
only where Untwist can show from the program text that it gives the same result the original
would.

Try it in the browser at **[untwist.dev](https://untwist.dev)**. The playground runs the same
engine as WebAssembly, and your code never leaves the page.

## What it does

- **Deobfuscate.** Finds javascript-obfuscator / obfuscator.io string arrays with their rotators,
  decoders (base64, RC4) and wrapper functions, and replaces each decoder call with the string it
  returns. It also removes the leftover machinery, inlines control-flow proxy objects, unflattens
  `switch` dispatch loops, strips self-defending and debug-protection code, and folds constant
  expressions.
- **Unminify.** Undoes minifier idioms: `!0` becomes `true`, comma sequences become statements,
  ternaries become `if`/`else`, Yoda conditions are flipped, `void 0` becomes `undefined`, and so on.
- **Untranspile.** Turns TypeScript, Babel, SWC and esbuild helpers for spread, destructuring,
  classes, generators and `async` back into native syntax.
- **Unpack.** Detects webpack 4/5 (including JSONP chunks), browserify, esbuild and Bun, rolldown
  (also Vite 8), SystemJS, AMD, UMD, Parcel, Metro, Turbopack and Closure bundles, extracts their modules and rewrites
  `require`/`import` specifiers to relative paths. Rollup and Vite scope-hoisted bundles can be
  split on request. Several input files can be unpacked together. The unpacked tree is untrusted
  code: its file names and contents come from the bundle. Do not open it with an editor or tool
  that runs project configuration on its own.
- **Rename.** Optional: `mangle` (short names, compatible with webcrack's `-m`), `simple` (short names per function),
  `unique` (readable, program-wide names such as `fn1`, `arr2`, `handler3`) or `smart` (names from evidence: `i`/`j`
  for loop counters, `item`/`index` and `acc` for array callbacks, `err` in `catch`, `fs` for `require("fs")`,
  `canvas` for `document.createElement("canvas")`, `ctx`, `data`, destructured keys and the property a function is
  exported or stored under; also the property a value is read from or written to, constructors (`socket` for
  `new WebSocket`, `vector3`, `floats`), element ids and storage keys (`chatList` for `getElementById("chatList")`),
  event handlers and their events (`onKeydown(keyEvent)`), timer and frame callbacks, `fetch` responses and bodies,
  setter values, export aliases, `is*` names for functions that return booleans, PascalCase for functions only used
  with `new`, and a parameter's name when every call passes the same named value; a name is reused in sibling scopes
  and numbered only where it would clash, and human-written names are left alone; long hex tokens such as
  `b475796ed633d5fd0485` count as machine-made here and are renamed when evidence names them).
- **CommonJS wrappers.** In every mode, the lazy `requireX()` wrappers that `@rollup/plugin-commonjs` writes
  (`if (hasRequiredX) return x; hasRequiredX = 1; ...`), terser's inlined forms of them, rolldown's
  `__commonJSMin` factories and the plugin's interop helpers (`commonjsGlobal`, `getDefaultExportFromCjs`,
  `getAugmentedNamespace`, `commonjsRequire`) get their names back when minification or obfuscation took them:
  `requireModule3`, `hasRequiredModule3`, `module3`, or the module's own name when one survives. Only names change;
  every wrapper still runs on its first call. `--disable name-commonjs-wrappers` turns it off.
- **Libraries.** Recognises core-js 3 (`internals/*` and `modules/*`, every minor 3.0 to 3.51),
  regenerator-runtime, tslib, Babel helpers (`@babel/runtime` and inlined) and `@swc/helpers` by the exact shape
  of their functions, minified, mangled or obfuscated, and puts `// core-js 3.38–3.51: internals/global-this.js`
  above each matched module. With any `--rename` mode the matched bindings get the library's own names
  (`requireGetBuiltIn`, `getBuiltIn`, `isCallable`, ...); the report lists each library with its version range.
  `--disable library-names,library-rename` turns this off.
- **JSX.** Rebuilds JSX from `React.createElement` and the automatic `jsx`/`jsxs` runtime.

## Install

### CLI

```sh
cargo install untwist-cli
```

This installs the `untwist` binary. It needs Rust 1.98 or newer.

### Rust library

```sh
cargo add untwist
```

```rust
use untwist::{Options, Pipeline};

let output = Pipeline::new(Options::default()).run("var a = !0;");
println!("{}", output.code());
output.write_to("out-dir")?;
```

### npm (WebAssembly)

```sh
npm install untwist
```

```js
import init, { run } from 'untwist';

await init();
const result = run(source, '--unsafe', false);
console.log(result.stdout);
```

The package is the single-threaded WebAssembly build the playground uses; see its
[README](crates/untwist-wasm/npm/README.md) for the full API and the flag format.

## Usage

```
untwist input.js                 # print the result to stdout
untwist input.js -o out-dir      # write deobfuscated.js (and modules, if a bundle is found)
untwist a.js b.js -o out-dir     # unpack several files together
cat input.js | untwist           # read from stdin
```

Common flags:

| Flag | What it does |
| --- | --- |
| `-o, --output DIR` | Write `deobfuscated.js`, and, when a bundle is found, `bundle.json` and one file per module. With several inputs each is written as `deobfuscated.<name>.js`; when only some of them form the bundle (a chunk unpacked without its loader, or an entry whose chunks are not bundles), every input is written under its own relative name instead, so the tree runs as the inputs did, and no module takes one of those names. |
| `-f, --force` | Overwrite the files in `DIR` that this run writes and remove the ones a previous run left there. Input files are never overwritten. |
| `--unsafe` | Use the unsafe lane (see below). |
| `--rename MODE` | `none` (default), `mangle`, `simple`, `smart` or `unique`. `-m` is short for `--rename mangle`. |
| `--no-deobfuscate`, `--no-unminify`, `--no-unpack`, `--no-jsx` | Skip a stage. |
| `--split-hoisted` | Also split Rollup/Vite scope-hoisted bundles, when the split provably keeps behaviour. |
| `--split-assume-modules` | With `--split-hoisted`: assume the split tree is ES modules for reading and for running in node, so `document.currentScript`, classic-script load timing and sloppy-mode top-level semantics are not preserved. Lifts the `currentScript` refusal, splits a bundle that sits inside an outer wrapper function, and makes every Rollup commonjs wrapper its own module while everything else stays in the entry. See [known divergences](docs/known-divergences/unpack.md). |
| `--single-file` | Write every unpacked module into one file, with a banner per module. |
| `--source-map [file\|inline]` | Emit a source map from the output back to the input. `file` (default, needs `-o`) writes `<file>.map` next to every written file with a `sourceMappingURL` comment; `inline` embeds the map as a data URL. Unpacked modules get one map each. The code itself is the same with or without the flag. |
| `--input-source-map PATH\|inline` | With `--source-map`: the input already has a source map (a file, or its inline data URL); map through it to the original sources. A source that is absolute, has a scheme or climbs out with `..` is written as `input-map:///<name>`; the input map's `sourceRoot` is joined into each source first and not copied. |
| `--wasm-check off\|on\|auto` | Look for wasm modules embedded in the output. `off` (default) only prints a note when the output may hold one; `auto` finds a byte array or base64 text that holds a whole module with imports or exports, writes it to `wasm/` and names its glue (feature-detect probes are left alone); `on` reparses every output to look, which can be slow on large files. |
| `--wasm-dir DIR`, `--wasm NAME=PATH` | Supply wasm modules the input loads from elsewhere, so their import and export names can name the glue. This works with every `--wasm-check` mode. |
| `--fetch-wasm`, `--wasm-base URL` | Download the wasm modules nobody supplied. Nothing is fetched without this flag. The URLs come from the input, so each one is printed before it is requested, and one run fetches at most 16 modules, 64 MiB and for 120 seconds. Redirects are followed three times at most. A host that resolves to a loopback, link-local or private address is refused, and no proxy from the environment is used. |
| `--fetch-wasm-local` | With `--fetch-wasm`, also fetch from loopback, link-local and private addresses, for input that loads its wasm from your own machine or network. |
| `--disable PASS` | Turn off individual passes; `--list-passes` prints them all. |
| `--report text\|json` | Print what every stage and pass did. `-v` is short for `--report text`. |
| `-j, --jobs N` | Process up to `N` inputs in parallel. The output does not depend on `N`. |
| `--max-memory SIZE` | Memory the inputs being processed may use (the texts the run holds are not counted), such as `512MiB`, `8G` or `unlimited` (default `4GiB`). At three quarters of it an input stops its passes and keeps the output so far, with the note `budget-memory`; at the full size that input is stopped and passed through unchanged, whatever its passes are doing. `--report` shows the budget and the peak. |
| `--timeout SECS` | Wall-clock seconds processing the inputs may take (default: no limit); writing the output and `--fetch-wasm` are not limited by it. At the limit every input stops its passes and keeps the output so far, with the note `budget-time`; an input still running a while later is passed through unchanged. |
| `--fuel N` | Interpreter steps a string-array decode may spend before giving up (default 50,000,000 plus 500,000 per distinct decoder call, at most 1,000,000,000; 1,000,000,000 with `--unsafe`). All decodes of one input share a pool of 32 times that per-decode figure or 2,000 steps per input byte, whichever is larger; once it is spent the remaining decodes are left encoded with a note. |

When an input stops its passes at a budget and keeps the output so far, that output still runs like the input: every rewrite in it was proven on its own.

`untwist --help` lists every flag.

### WebAssembly

untwist never runs or rewrites a wasm module; it reads only its import and export names (and its
name section, when it has one) to name the JS glue around it. Each name is taken only after a
check that every import key and export read in the glue exists in the module; a module that does
not match gives a warning and names nothing. Embedded modules are found with `--wasm-check`, and
modules loaded from elsewhere are supplied with `--wasm-dir`, `--wasm` or `--fetch-wasm`. An
embedded module is written to `wasm/` and its literal stays in the code; only a `data:` URI passed
straight to `fetch(...)` is replaced by the path of the written file. In a classic script the
glue's top-level `var` and function names keep their names, because other scripts can reach
them, and so do a sloppy-mode block function and the `var` of the same name that it writes (or
code outside the block that reads the function's name). In a
webpack bundle the names reach the module files that import from or export to the wasm module,
and the same names are applied to the combined `deobfuscated.js`. That includes wasm-bindgen
glue (bundler target) in webpack 4 and 5 bundles that reaches the module through
`import * as wasm`, directly or handed over with `__wbg_set_wasm`: a wrapper that calls one
export, which no other function calls, takes that export's name. Glue and module given as
separate input files are not linked. For Emscripten glue whose
module has no name section, exports are named from `Module._name` wrappers and the export kind
(`wasmMemory`, `wasmTable`). Minified import names like `a.a` say nothing about the function
behind them, so an import is named only when its function has the shape of an Emscripten library
function (`HEAPU8.copyWithin(dest, src, src + num)` is `_emscripten_memcpy_big`) and the module's
import has that function's signature. Such a module must also have every import the glue
provides and no import the glue leaves out; otherwise nothing is named.

Exit codes: `0` success; `1` an error, a parse error (output is still produced) or a panic in a
pass; `2` a file this run would write already exists in the output directory (use `-f`); `3` an
input was too deep to process safely and was passed through unchanged.

On Unix the pipeline runs in a worker process, so an input that crashes it (a stack overflow or a
failed allocation) is passed through and the rest of the run is kept. Windows has no worker: the
memory and time limits still stop an input and pass it through, but a crash ends the whole run
without output.

## Safe and unsafe lanes

By default Untwist runs in the **safe lane**. A rewrite is made only when it can be proven from
the program text. A string-array call is decoded only if nothing in the program can change what
the decoder returns at that point: no patched built-in it relies on, no alias that reaches the
array or decoder before the rotator runs, no code that is built at run time from strings. When
the proof fails, the call stays in place and the report says why. The output behaves like the
input.

The safe lane rests on a few stated premises about code it cannot see. One input file is a closed
program. Several classic scripts given in one run share one global scope, as scripts on one page
do: a top-level `var`, function, `let`, `const` or `class` of one is treated as read and written by
the others when another input names it (a free reference or a top-level declaration of its own) or
reaches globals through a computed key or code text, and is then not inlined away, removed or
renamed. A built-in name (`eval`, `String`, `atob`, ...) that another input declares at its top level or assigns is not the built-in for any input, so no fold relies on it. Inputs that share no names come out as they would alone. A module (a file with `import`, `export`,
`import.meta` or top-level `await`) is closed apart from its exports. Code from outside the file
(`require` results, host functions, other scripts) writes no built-in. It also does not depend on
the names the obfuscator picked for the string array and its decoders, because that code was
written before those names existed. So when such code is handed a value that may hold the global
object, the machinery can still be removed. Outside code that prints that value still counts as
observing it (`require("util").inspect(globalThis)` keeps the machinery). A removal that relies
on this premise says so in the report (`the machinery was dropped under the outside-code
premise`). Nor does any code give `Object.prototype` an accessor for a key the obfuscator
generated or for a webpack module id, so storing an object under such a key runs no code. And
no value the program tests is `document.all`, so `x != null ? x : y` becomes `x ?? y`: the two
differ only for that object, which compares loosely equal to `null` without being nullish.
[`docs/known-divergences/deobfuscate.md`](docs/known-divergences/deobfuscate.md) lists every
premise.

With `--rename`, the safe lane keeps the name of every function and class when the file reads
`.name` with a static key, since renaming them would change what it reads. A read through a
computed key (`f[key]`) or through reflection is not seen, as `fn.toString()` is not;
[`docs/known-divergences/rename.md`](docs/known-divergences/rename.md) has the details.

The **unsafe lane** (`--unsafe`) makes the assumptions webcrack makes: built-in objects are not
patched, even when the program visibly patches one, a direct `eval` reads and writes no local
variable (the lexical passes still refuse a program whose `eval` may), and a few passes that rest on
heuristics are allowed to run. These are `template-literals` and JSX reconstruction (`jsx`, `jsx-new`).
`dead-code`, which deletes branches behind constant comparisons, runs in both lanes.
`fold-pure-calls` also folds calls to a pure helper that is passed around as a value or that the
program could rebind through a key it cannot resolve.
A Metro bundle whose file does not hold Metro's own `__d`/`__r` runtime is unpacked on the trust
that those names are Metro's.
This decodes more on aggressively protected code. It is the right choice when you trust that the
input does not tamper with the runtime, and the wrong one for code written to fool analysis.

Every deliberate difference from webcrack is listed, with the reason, in
[`docs/known-divergences.md`](docs/known-divergences.md), where webcrack is called the reference
deobfuscator.

## Playground

[untwist.dev](https://untwist.dev) runs Untwist in a Web Worker. Paste, drop or open a script and
press Run. You can browse and download the unpacked modules, compare input and output side by side,
read diagnostics and the run report, and edit the output with go to definition, find references
and rename. The output is byte for byte what the CLI gives for the same flags. The source is in
[`web/`](web/README.md).

## Compared with webcrack

| | Untwist | webcrack |
| --- | --- | --- |
| Language | Rust (oxc) | TypeScript (Babel) |
| Runs the input's code | Never; decoders run in a sandboxed, budgeted interpreter | Runs decoders in `isolated-vm` |
| Decoding under tampering | Refuses what it cannot prove, by default | Assumes built-ins are unpatched |
| Bundlers | webpack, browserify, esbuild/Bun, rolldown/Vite 8, SystemJS, AMD, UMD, Parcel, Metro, Turbopack, Closure, Rollup/Vite (opt-in) | webpack, browserify |
| Untranspile helpers | TypeScript, Babel, SWC, esbuild | No |
| Rename modes | `mangle`, `simple`, `unique` | `mangle` |

Untwist's output is checked against webcrack's with differential harnesses (`cargo xtask
diff-unminify`, `diff-deobfuscate`, `diff-unpack`, `diff-mangle`, `diff-jsx`), which run Untwist in
an internal comparison mode limited to the passes webcrack has. Decoded output is also checked by
running both the input and the output under Node and comparing what they print. `cargo xtask
obf-options` runs javascript-obfuscator 5.9.1 output for every free option, pairs of the
high-impact ones and the four presets through both lanes and a pinned webcrack, and requires each
output to keep no obfuscator machinery and to print what the original program prints.
`--suggest-expect <file>` writes a whole replacement for its `EXPECT`. Each failure is named by its
cause, and each cause keeps the reason it already has in `EXPECT`.
`cargo xtask obf-options --dir fixtures/obfuscator-versions` runs the same checks on the version
matrix that `tools/obfuscated/versions.mjs` generates: twenty javascript-obfuscator releases from
0.10.2 to 5.9.1, each with its own string array options (encoding, rotate, shuffle, index shift,
wrappers, calls transform, threshold) and one extra transform, placed at the top level, in an IIFE,
in a webpack 4 module or around a whole webpack 4 bundle, then left as is or minified with
uglify-js or terser. The cells cover every version × encoding × container × minifier and every
pair of the other choices. Its `EXPECT` says for each cell whether the run is refused with a named
reason or `MISSED` (no string array found, or calls left with no note).
`cargo xtask obf-fuzz --seed <n> --count <n>` draws random programs from a grammar of those
features, obfuscates them with random option sets and rewires the decoder (aliases, wrappers, inert
statements, strict mode, a wrapping function), then judges both lanes the same way. A failure is
grouped by the cause key of its first blocking note and shrunk to a small repro. Only causes listed in
`fixtures/fuzz/EXPECT` (open gaps) or `fixtures/obfuscator-io/SOUND_REFUSALS` (refusals that are the
right answer, on programs with the named feature) are allowed.

## Building from source

```sh
cargo build --release -p untwist-cli    # target/release/untwist
cargo test --workspace
```

The workspace layout:

| Crate | Contents |
| --- | --- |
| `untwist` | The pipeline and the library API |
| `untwist-cli` | The `untwist` command-line tool |
| `untwist-core` | Parsing, printing and the pass framework |
| `untwist-eval` | Constant folding and the sandboxed interpreter |
| `untwist-deobfuscate` | String arrays, control flow and protections |
| `untwist-unminify` | Unminify and untranspile passes |
| `untwist-unpack` | Bundle detection and module extraction |
| `untwist-rename` | Identifier renaming |
| `untwist-jsx` | JSX reconstruction |
| `untwist-wasm` | The WebAssembly build (npm `untwist`) |
| `untwist-regress` | A fork of [regress](https://github.com/ridiculousfish/regress) with a step-budgeted search |

Some tests read a large real-world sample at `fixtures/real-world/game.js`, which is not
distributed with the repository; they are skipped when it is absent. The `cargo xtask diff-*`
harnesses need a built webcrack clone in `reference/` (or set `UNTWIST_REFERENCE_DIR`) and a CLI
built with `cargo build --release -p untwist-cli --features test-hooks`; `cargo xtask obf-options
--webcrack` also needs `npm ci --prefix tools/webcrack`. Test inputs are described
in [`fixtures/README.md`](fixtures/README.md).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at
your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.

Untwist's behaviour follows webcrack, and the fixtures under `fixtures/reference/` come from
webcrack (MIT, Copyright (c) 2023 j4k0xb, see [`fixtures/LICENSE-reference`](fixtures/LICENSE-reference)).
`vendor/regress` is a fork of regress by ridiculousfish (MIT OR Apache-2.0). See [`NOTICE`](NOTICE).
