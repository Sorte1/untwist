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
- **Unpack.** Detects webpack 4/5 (including JSONP chunks), browserify, esbuild and Bun, SystemJS,
  AMD, UMD, Parcel, Metro, Turbopack and Closure bundles, extracts their modules and rewrites
  `require`/`import` specifiers to relative paths. Rollup and Vite scope-hoisted bundles can be
  split on request. Several input files can be unpacked together.
- **Rename.** Optional: `mangle` (short names, compatible with webcrack's `-m`), `simple` (short names per function) or
  `unique` (readable, program-wide names such as `fn1`, `arr2`, `handler3`).
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
| `-o, --output DIR` | Write `deobfuscated.js`, and, when a bundle is found, `bundle.json` and one file per module. |
| `-f, --force` | Replace the files a previous run wrote to `DIR`. |
| `--unsafe` | Use the unsafe lane (see below). |
| `--rename MODE` | `none` (default), `mangle`, `simple` or `unique`. `-m` is short for `--rename mangle`. |
| `--no-deobfuscate`, `--no-unminify`, `--no-unpack`, `--no-jsx` | Skip a stage. |
| `--split-hoisted` | Also split Rollup/Vite scope-hoisted bundles, when the split provably keeps behaviour. |
| `--single-file` | Write every unpacked module into one file, with a banner per module. |
| `--wasm-dir DIR`, `--wasm NAME=PATH` | Supply wasm modules the input loads from elsewhere, so their import and export names can name the glue. Embedded wasm is written to `wasm/` on its own. |
| `--fetch-wasm`, `--wasm-base URL` | Download the wasm modules nobody supplied. Nothing is fetched without this flag. |
| `--disable PASS` | Turn off individual passes; `--list-passes` prints them all. |
| `--report text\|json` | Print what every stage and pass did. `-v` is short for `--report text`. |
| `-j, --jobs N` | Process up to `N` inputs in parallel. The output does not depend on `N`. |
| `--fuel N` | Interpreter steps a string-array decode may spend before giving up (default 50,000,000, or 1,000,000,000 with `--unsafe`). |

`untwist --help` lists every flag.

Exit codes: `0` success; `1` an error, a parse error (output is still produced) or a panic in a
pass; `2` the output directory already holds output (use `-f`); `3` an input was too deep to
process safely and was passed through unchanged.

## Safe and unsafe lanes

By default Untwist runs in the **safe lane**. A rewrite is made only when it can be proven from
the program text. A string-array call is decoded only if nothing in the program can change what
the decoder returns at that point: no patched built-in it relies on, no alias that reaches the
array or decoder before the rotator runs, no code that is built at run time from strings. When
the proof fails, the call stays in place and the report says why. The output behaves like the
input.

The safe lane rests on a few stated premises about code it cannot see. Code from outside the file
(`require` results, host functions, other scripts) writes no built-in. It also does not depend on
the names the obfuscator picked for the string array and its decoders, because that code was
written before those names existed. So when such code is handed a value that may hold the global
object, the machinery can still be removed. Outside code that prints that value still counts as
observing it (`require("util").inspect(globalThis)` keeps the machinery). A removal that relies
on this premise says so in the report (`the machinery was dropped under the outside-code
premise`). [`docs/known-divergences/deobfuscate.md`](docs/known-divergences/deobfuscate.md)
lists every premise.

The **unsafe lane** (`--unsafe`) makes the assumptions webcrack makes: built-in objects are not
patched, even when the program visibly patches one, and a few passes that rest on heuristics are
allowed to run. These are `dead-code` (deletes branches behind constant comparisons),
`template-literals`, and JSX reconstruction. `fold-pure-calls` also folds calls to a pure helper
that is passed around as a value or that the program could rebind through a key it cannot resolve.
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
| Bundlers | webpack, browserify, esbuild/Bun, SystemJS, AMD, UMD, Parcel, Metro, Turbopack, Closure, Rollup/Vite (opt-in) | webpack, browserify |
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
