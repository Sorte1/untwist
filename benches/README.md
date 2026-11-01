# Benchmarks

Benchmarks and memory probes. Figures depend on the machine and move by a few percent between runs,
so read them as the shape of the cost, not exact values.

## Criterion

```sh
cargo bench -p untwist                    # unminify and deobfuscate groups
cargo bench -p untwist -- deobfuscate     # one group
cargo bench -p untwist-eval --bench decode
```

`crates/untwist/benches/unminify.rs` times the pipeline over small, medium and large fixtures, with
unminify only and with the full deobfuscator. `crates/untwist-eval/benches/decode.rs` times the
string-array decoder's instantiate, rotate and read steps. Reports land in `target/criterion/`.

## Against the reference deobfuscator

`baseline-reference.json` holds the reference deobfuscator's wall-clock time in milliseconds,
Node startup included, for each file under `fixtures/reference/`. With a built clone in `reference/`
(or wherever `UNTWIST_REFERENCE_DIR` points):

```sh
cargo xtask bench-baseline                                # rewrite the baseline
cargo build --release -p untwist-cli --features test-hooks
cargo xtask bench-compare [filter]                        # speedup table, 20 runs per file
```

## Memory

- `cargo bench -p untwist --bench memory -- <file.js> [--deobfuscate | --floor | --nodes]` prints
  the counting allocator's peak for one input.
- `cargo bench --no-run -p untwist-cli --features peak-probe --bench peak` builds the CLI with a
  resident-memory probe that prints a `peak` line on stderr.
- `benches/peak.sh <rounds> <label=binary>... -- <untwist args...>` runs probe binaries under
  several malloc policies; `benches/rss-split.sh <binary> <args...>` splits the peak into heap,
  anonymous and file-backed memory.

Fuzzing is described in [`fuzz/README.md`](../fuzz/README.md).
