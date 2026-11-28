# Fuzzing

`cargo-fuzz` targets for the pipeline, the passes and the evaluator. The crate is excluded from the
workspace because it needs nightly and `libfuzzer-sys`.

| Target | What it checks |
|---|---|
| `pipeline` | `untwist::Pipeline::run` with and without unsafe transforms never panics |
| `passes` | every pass in every pass group survives any parse, error-recovered trees included |
| `evaluator` | the deobfuscator respects its round cap |
| `options` | the pipeline never panics under the option sets the first three bytes pick (`--unsafe`, `--restore svelte/solid`, `--library-imports`, module/script/CommonJS goal, every rename mode, `--types`, `--modern`, an input source map after `\n//# fuzz-map\n`) and plans no file outside the output directory |
| `libraries` | a library collection built from the input (index, gzip shards, bloom filter, or a raw `collection.json`) attaches and the pipeline runs against it without panicking |

```sh
sh fuzz/corpus_seed.sh
cargo +nightly fuzz run pipeline --target x86_64-unknown-linux-gnu -- -max_total_time=300 -timeout=120 -fork=1
```

`corpus_seed.sh` seeds `fuzz/corpus/<target>/` from every fixture under 512 KB (and the Svelte and
Solid test inputs) except `fixtures/bla`; `options` seeds get a flag prefix that fits the fixture,
`libraries` seeds are built from `fixtures/library-collection`. The `options` and `libraries`
logic lives in `fuzz/harness/`, which the regression test includes too. The interpreter is
slow under AddressSanitizer, hence `-timeout=120`; `-fork=1` keeps a campaign going past a slow input.

Crashing inputs are committed under `fuzz/artifacts/<target>/`.
`crates/untwist/tests/fuzz_regressions.rs` replays each one through the configuration of the
target that found it, so `cargo test --workspace` fails if a crash comes back. `timeout-` and
`slow-unit-` inputs run only in release builds.
