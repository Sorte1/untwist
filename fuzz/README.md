# Fuzzing

`cargo-fuzz` targets for the pipeline, the passes and the evaluator. The crate is excluded from the
workspace because it needs nightly and `libfuzzer-sys`.

| Target | What it checks |
|---|---|
| `pipeline` | `untwist::Pipeline::run` with and without unsafe transforms never panics |
| `passes` | every pass in every pass group survives any parse, error-recovered trees included |
| `evaluator` | the deobfuscator respects its round cap |

```sh
sh fuzz/corpus_seed.sh
cargo +nightly fuzz run pipeline -- -max_total_time=300 -timeout=120 -fork=1
```

`corpus_seed.sh` seeds `fuzz/corpus/<target>/` from every fixture under 512 KB. The interpreter is
slow under AddressSanitizer, hence `-timeout=120`; `-fork=1` keeps a campaign going past a slow input.

Crashing inputs are committed under `fuzz/artifacts/<target>/`.
`crates/untwist/tests/fuzz_regressions.rs` replays each one through the configuration of the
target that found it, so `cargo test --workspace` fails if a crash comes back. `timeout-` and
`slow-unit-` inputs run only in release builds.
