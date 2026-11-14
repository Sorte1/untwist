# Fixtures

Inputs for the test suite and the `cargo xtask` differentials. A program that "prints" writes its
own observables, so node must print the same thing before and after Untwist.

| Folder | What it holds | Used by |
|---|---|---|
| `behaviour/` | Programs that print, one string-array, rotator or decoder hazard each; `.mjs` files run as modules | `cargo xtask diff-behaviour`, `crates/untwist/tests/decode_behaviour.rs` |
| `bundle-matrix/` | Two small apps (CommonJS deps, a CommonJS cycle, JSON, a dynamic import; core-js polyfills from Babel `useBuiltIns`) built by every bundler, version and runtime-shaping option in `tools/bundle-matrix/`, then minified or obfuscated; `MANIFEST`, `SHA256SUMS`, `EXPECT` and `expected/` describe each cell, `DROPPED` lists the cells whose own output does not run | `cargo xtask bundle-matrix` (local and release check), `xtask/tests/bundle_matrix_drift.rs` |
| `browserify-preludes/` | One small CommonJS app packed by browser-pack 0.9, 2, 3, 5, 6.0 and 6.1 (plain, two entries, `--standalone`, `hasExports`) and minified by terser, uglify-js, esbuild and swc, from `tools/browserify-preludes/gen.mjs`; `expected/` holds each cell's stdout | `crates/untwist-cli/tests/browserify_preludes.rs`, the recorded forms in `crates/untwist-unpack/src/browserify/forms.rs` |
| `builtin-replaced/` | Programs that print, a string-array unit beside a write to a built-in | `crates/untwist/tests/builtin_replaced_behaviour.rs` |
| `corpus-npm/` | Generated npm-style modules obfuscated with several decoder shapes | `cargo xtask corpus-npm`, `crates/untwist-deobfuscate/tests/corpus_npm.rs` |
| `dead-code/` | Programs that print, one rule of the safe `dead-code` fold each | `crates/untwist/tests/dead_code_behaviour.rs` |
| `deobfuscate-gaps/` | Synthetic inputs for known deobfuscation gaps | `crates/untwist-deobfuscate/tests/` |
| `eval/` | Decoder corpus over every array, rotator, decoder and encoding combination; do not hand-edit | `cargo xtask corpus`, `crates/untwist-eval/tests/` |
| `generated/` | Large generated bundles, not committed | `cargo xtask corpus-bundle`, the big-bundle tests |
| `jsx/` | `createElement` and `jsx()` call shapes | `cargo xtask diff-jsx`, `crates/untwist-jsx/tests/fixtures.rs` |
| `layers/` | `eval`, `Function`, `atob` and p.a.c.k.e.r. layers, as classic scripts that print | `cargo xtask diff-layers`, `crates/untwist/tests/layers_*.rs` |
| `modern/` | Babel preset-env + core-js 3.39 builds (usage and entry, targets ie 11) through Rollup, webpack and esbuild from `tools/modern-fixtures/build.mjs`, as programs that print; `MANIFEST` lists each | `crates/untwist/tests/modern_fixtures.rs` |
| `obfuscated/` | javascript-obfuscator output from `tools/obfuscated/gen.mjs`; `MANIFEST` and `EXPECT` describe each file | `cargo xtask obf-matrix` |
| `obfuscators/` | Output of obfuscators, encoders and minifiers outside the javascript-obfuscator matrices (Closure Compiler, JSFuck, aaencode, jjencode, js-confuser, js-virtualizer, terser and uglify property mangling, webpack's eval devtool, eval/Function loaders), from `tools/obfuscators/gen.mjs`; `MANIFEST` records each file's tool, options and the outcome per lane | `crates/untwist/tests/other_obfuscators.rs`, `node tools/obfuscators/check.mjs` |
| `obfuscator-io/` | javascript-obfuscator 5.9.1 output over every free option, pairs of the high-impact options and the four presets, from `tools/obfuscated/matrix.mjs`; `MANIFEST`, `EXPECT`, `ORACLE` (webcrack's results) and `BASELINE` describe each file | `cargo xtask obf-options`, `xtask/tests/obf_options_drift.rs` |
| `pure-calls/` | Programs that print, a pure helper called with literal arguments that `fold-pure-calls` folds, or a near miss it must leave alone | `crates/untwist/tests/pure_calls_behaviour.rs` |
| `real-shapes/` | Obfuscator and webpack machinery reduced from real bundles by `tools/real-shapes/reduce.mjs` | `cargo xtask obf-matrix` |
| `real-world/` | Optional local `game.js` and its reference snapshot, not committed; `game-truth.mjs` prints its array truth | the `real_world` tests, which skip when it is absent |
| `reference/` | Samples and expected output from the reference deobfuscator (MIT, `LICENSE-reference`) | `cargo xtask diff-deobfuscate`, `diff-unpack`, `crates/untwist-deobfuscate/tests/fixtures.rs` |
| `rollup-commonjs/` | `@rollup/plugin-commonjs` 11 to 29, Vite 5 to 8 and rolldown output over synthetic CommonJS apps, core-js and npm packages, plain, minified (terser, esbuild, swc) and obfuscated (javascript-obfuscator), from `tools/rollup-commonjs/gen.mjs`; `MANIFEST` lists each, `EXPECT` the wrappers found per lane | `crates/untwist/tests/rollup_commonjs.rs` |
| `spelled-name/` | Programs that print, whose machinery names are also spelled as property keys | `crates/untwist/tests/spelled_name_behaviour.rs` |
| `unsafe-parity/` | javascript-obfuscator output that the reference deobfuscator decodes and the safe lane refuses; `MANIFEST` names each source under `sources/` and the options | `crates/untwist/tests/unsafe_parity.rs` |
| `unminify/` | Minified programs for the unminify passes | `cargo xtask diff-unminify`, `diff-mangle` |
| `unpack-a37/` | The round-three review probes for closed evals in unpack, webpack 4 module polyfills and declared prototype methods (hand-written programs around webpack 4.47.0 runtimes; `m6/` ES modules for `--unsafe --rename smart`), each printing what it computes; `k*.js` carry the same `.caller` route into other bundle kinds | `crates/untwist/tests/unpack_a37_rows.rs` |
| `unpack-bundlers/` | Bundler output from `tools/bundlers/` plus hand-written `min-*` and `neg-*` shapes; `MANIFEST` lists each | `cargo xtask diff-unpack-behaviour`, `crates/untwist-unpack/tests/` |
| `unpack-rolldown/` | rolldown 1.x and Vite 8 output from `tools/rolldown/gen.mjs` (formats, minify, code splitting, keepNames, treeshake) with each cell's stdout; `EXPECT` and `XFAIL` | `crates/untwist-cli/tests/rolldown_tree.rs` |
| `unpack-gaps/` | Hand-written bundles for known unpacking gaps | `crates/untwist-unpack/tests/` |
| `untranspile/` | TypeScript, swc, Babel, esbuild and terser output from `tools/untranspile/gen.mjs` (`params/` from `tools/untranspile-params/gen.mjs`), as programs that print | `cargo xtask diff-behaviour`, `crates/untwist-unminify/tests/` |
| `webpack-buildin/` | webpack 4.47.0 bundles whose modules go through `buildin/module.js` and `buildin/harmony-module.js` (a UMD wrapper, `.loaded` reads, an ESM module handing `module` on; `w4-negative-*` read `.id`, `.children` or a harmony `.exports`), dev, production and minified, web and node, from `tools/webpack-variants/buildin.mjs`, each with its stdout; `MANIFEST` lists each | `crates/untwist/tests/webpack_buildin.rs` |
| `webpack-eval/` | webpack 4/5 output in every `eval*` devtool, the development default, production with `devtool: eval` and the web target, from `tools/webpack-eval/gen.mjs`; `MANIFEST` lists each | `crates/untwist/tests/webpack_eval.rs` |
| `webpack-soundness/` | Hand-edited webpack 5.112 inputs (stock runtime from `webpack-variants/`, own entry, chunk or wrapper) that a merged tree would run differently; `EXPECT` says refused or runs | `crates/untwist/tests/webpack_soundness.rs`, which runs input and tree in node |
| `write-tree/` | Multi-file bundler output (an entry with a lazy chunk) copied from the `bundle-matrix/` cells of the same name, `tools/bundle-matrix/`; `expected/` holds what the original prints | `crates/untwist-cli/tests/write_tree.rs`, which runs the `-o` tree in node |

Third-party inputs keep their licence beside them: `LICENSE-reference`,
`LICENSE-javascript-obfuscator`, `obfuscators/licenses/`, `unpack-bundlers/licenses/`, `unpack-rolldown/licenses/`, `bundle-matrix/licenses/`, `webpack-eval/LICENSE-webpack`, `webpack-soundness/LICENSE-webpack` and `write-tree/licenses/`.

Not every sample in `reference/deobfuscate/` comes from the reference deobfuscator.
`obfuscator.io-site-memo-reset.js` is the obfuscator.io website's output for `console.log("")`.
`obfuscator.io-memo-reset-*.js` are javascript-obfuscator 5.9.0 output for the `hi()` program
the other samples print, with a string array encoded as base64, rc4, or both together with
`selfDefending` and `debugProtection`. Both kinds carry the decoder memo reset gate that
javascript-obfuscator 5 emits. Their `.snap` files hold the program before obfuscation.

## Adding a fixture

Put the file in the folder whose test should pick it up, name it after the shape it covers, and make
it print what it computes so node can check it. Generated folders (`eval/`, `corpus-npm/`,
`obfuscated/`, `obfuscator-io/`, `obfuscators/`, `rollup-commonjs/`, `unpack-bundlers/`, `unpack-rolldown/`, `bundle-matrix/`, `untranspile/`, `webpack-buildin/`) are rewritten by their generator, so change the
generator instead. Run the folder's test or differential; add an allowlist entry only for a
divergence documented in `docs/known-divergences/`.
