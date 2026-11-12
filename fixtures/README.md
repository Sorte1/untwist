# Fixtures

Inputs for the test suite and the `cargo xtask` differentials. A program that "prints" writes its
own observables, so node must print the same thing before and after Untwist.

| Folder | What it holds | Used by |
|---|---|---|
| `behaviour/` | Programs that print, one string-array, rotator or decoder hazard each; `.mjs` files run as modules | `cargo xtask diff-behaviour`, `crates/untwist/tests/decode_behaviour.rs` |
| `builtin-replaced/` | Programs that print, a string-array unit beside a write to a built-in | `crates/untwist/tests/builtin_replaced_behaviour.rs` |
| `corpus-npm/` | Generated npm-style modules obfuscated with several decoder shapes | `cargo xtask corpus-npm`, `crates/untwist-deobfuscate/tests/corpus_npm.rs` |
| `dead-code/` | Programs that print, one rule of the safe `dead-code` fold each | `crates/untwist/tests/dead_code_behaviour.rs` |
| `deobfuscate-gaps/` | Synthetic inputs for known deobfuscation gaps | `crates/untwist-deobfuscate/tests/` |
| `eval/` | Decoder corpus over every array, rotator, decoder and encoding combination; do not hand-edit | `cargo xtask corpus`, `crates/untwist-eval/tests/` |
| `generated/` | Large generated bundles, not committed | `cargo xtask corpus-bundle`, the big-bundle tests |
| `jsx/` | `createElement` and `jsx()` call shapes | `cargo xtask diff-jsx`, `crates/untwist-jsx/tests/fixtures.rs` |
| `layers/` | `eval`, `Function`, `atob` and p.a.c.k.e.r. layers, as classic scripts that print | `cargo xtask diff-layers`, `crates/untwist/tests/layers_*.rs` |
| `obfuscated/` | javascript-obfuscator output from `tools/obfuscated/gen.mjs`; `MANIFEST` and `EXPECT` describe each file | `cargo xtask obf-matrix` |
| `obfuscator-io/` | javascript-obfuscator 5.9.1 output over every free option, pairs of the high-impact options and the four presets, from `tools/obfuscated/matrix.mjs`; `MANIFEST`, `EXPECT`, `ORACLE` (webcrack's results) and `BASELINE` describe each file | `cargo xtask obf-options`, `xtask/tests/obf_options_drift.rs` |
| `pure-calls/` | Programs that print, a pure helper called with literal arguments that `fold-pure-calls` folds, or a near miss it must leave alone | `crates/untwist/tests/pure_calls_behaviour.rs` |
| `real-shapes/` | Obfuscator and webpack machinery reduced from real bundles by `tools/real-shapes/reduce.mjs` | `cargo xtask obf-matrix` |
| `real-world/` | Optional local `game.js` and its reference snapshot, not committed; `game-truth.mjs` prints its array truth | the `real_world` tests, which skip when it is absent |
| `reference/` | Samples and expected output from the reference deobfuscator (MIT, `LICENSE-reference`) | `cargo xtask diff-deobfuscate`, `diff-unpack`, `crates/untwist-deobfuscate/tests/fixtures.rs` |
| `spelled-name/` | Programs that print, whose machinery names are also spelled as property keys | `crates/untwist/tests/spelled_name_behaviour.rs` |
| `unsafe-parity/` | javascript-obfuscator output that the reference deobfuscator decodes and the safe lane refuses; `MANIFEST` names each source under `sources/` and the options | `crates/untwist/tests/unsafe_parity.rs` |
| `unminify/` | Minified programs for the unminify passes | `cargo xtask diff-unminify`, `diff-mangle` |
| `unpack-bundlers/` | Bundler output from `tools/bundlers/` plus hand-written `min-*` and `neg-*` shapes; `MANIFEST` lists each | `cargo xtask diff-unpack-behaviour`, `crates/untwist-unpack/tests/` |
| `unpack-gaps/` | Hand-written bundles for known unpacking gaps | `crates/untwist-unpack/tests/` |
| `untranspile/` | TypeScript, swc, Babel, esbuild and terser output from `tools/untranspile/gen.mjs` (`params/` from `tools/untranspile-params/gen.mjs`), as programs that print | `cargo xtask diff-behaviour`, `crates/untwist-unminify/tests/` |

Third-party inputs keep their licence beside them: `LICENSE-reference`,
`LICENSE-javascript-obfuscator` and `unpack-bundlers/licenses/`.

Not every sample in `reference/deobfuscate/` comes from the reference deobfuscator.
`obfuscator.io-site-memo-reset.js` is the obfuscator.io website's output for `console.log("")`.
`obfuscator.io-memo-reset-*.js` are javascript-obfuscator 5.9.0 output for the `hi()` program
the other samples print, with a string array encoded as base64, rc4, or both together with
`selfDefending` and `debugProtection`. Both kinds carry the decoder memo reset gate that
javascript-obfuscator 5 emits. Their `.snap` files hold the program before obfuscation.

## Adding a fixture

Put the file in the folder whose test should pick it up, name it after the shape it covers, and make
it print what it computes so node can check it. Generated folders (`eval/`, `corpus-npm/`,
`obfuscated/`, `obfuscator-io/`, `unpack-bundlers/`, `untranspile/`) are rewritten by their generator, so change the
generator instead. Run the folder's test or differential; add an allowlist entry only for a
divergence documented in `docs/known-divergences/`.
