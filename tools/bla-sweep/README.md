# tools/bla-sweep

A release gate for BLA (bla-agents) protection. It protects our own clean sources with the pinned
bla-agents 3.6.1 (`package-lock.json`), runs Untwist on each protected file in both lanes and
compares every output with the unprotected source under node.

```sh
npm ci --ignore-scripts --prefix tools/bla-sweep
CARGO_TARGET_DIR=$HOME/.cache/untwist-targets/bla27 untwist-build-slot cargo build --release -p untwist-cli
untwist-run-slot node tools/bla-sweep/sweep.mjs --untwist $HOME/.cache/untwist-targets/bla27/release/untwist --jobs 6
```

The generator and every protected file are untrusted. `gen.mjs` runs bla-agents' `protectTarget`
(and `encryptStrings` for the `encrypt-strings` variant) inside bubblewrap with every namespace
unshared, so it has no network, sees only `/usr`, this folder read-only, the one source file
read-only and an empty output folder. Every node run (the source, the protected file and both
outputs) happens in the same kind of sandbox under `timeout 30`. Untwist only reads the files.

The plan is 17 sources (the matrix sources from `tools/obfuscated`, `plain/script.cjs`, two
CommonJS fixtures, four library bundles from `fixtures/libraries` and `sources/license.cjs`) by
the four intensities (stealth, balanced, maximum, bankrupt) by four variants (`default`,
`no-honey-map`, `llm-directives`, `self-seal`), plus `encrypt-strings` for `license.cjs`: 276
cases. Each case has a fixed seed (`untwist-bla27/<case>`), so bla-agents writes the same bytes
every time; `DIGESTS` holds the SHA-256 of every protected file and the sweep fails when one
changes. Untwist runs with `--source-type commonjs`, which is how node runs these files.

Each lane gets one verdict per case, recorded in `EXPECT`:

- `perfect`: the output prints what the source prints and exits alike, and holds no
  `Math.imul(seed ^ k ...)` predicate and no identifier that bla-agents added.
- `kept`: the output behaves like the source, but BLA machinery is left.
- `WRONG`: the output behaves differently from the source. Any WRONG fails the sweep.

A case where the generator refuses the source or the protected file does not behave like the
source is `refused` or `broken`. The sweep fails when a verdict gets worse than `EXPECT` and
lists those that got better; `--write` records a new `EXPECT` and `DIGESTS` from a full run.
Results go to `~/.cache/untwist-tmp/bla-sweep` (`--out DIR`): `results.json`, `summary.json` and
`cases/<id>/` for every case that is not perfect in both lanes (`--keep` keeps all).
`--filter RE` runs the matching case ids.
