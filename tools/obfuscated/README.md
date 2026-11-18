# tools/obfuscated

Generators for the javascript-obfuscator fixtures. Install the pinned packages first:

```sh
npm ci --prefix tools/obfuscated
```

| Script | What it does |
|---|---|
| `gen.mjs` | writes `fixtures/obfuscated/` |
| `matrix.mjs` | writes `fixtures/obfuscator-io/`: every option alone, pairs of the high-impact options, the four presets |
| `relayer.mjs` | writes `fixtures/obfuscator-relayer/`: the same options applied one to four times |
| `versions*.mjs` | writes `fixtures/obfuscator-versions/` across older javascript-obfuscator lines |
| `sweep.mjs` | the option sweep release gate below; writes nothing into the repository |

## sweep.mjs

The sweep checks that every javascript-obfuscator 5 option combination goes through Untwist. It
reads the option list and each option's values from the package's own typings (`IOptions.d.ts` and
the enums it names), drops the options that do not change code (source maps, logging, seed and the
like), and splits thresholds into 0, 0.5 and 1, encodings into every subset, and the other numbers
and lists into a few values each. It asks the obfuscator which pairs of values it rejects, then
builds a pairwise covering array over the rest, adds the four presets and some seeded random full
combinations, and applies each one once, twice and three times over (the same options on every
layer, and mixed options per layer) to the matrix sources and `console.log("test")`.

For each case it runs node on the obfuscated file and skips the case when that does not print what
the source prints (the obfuscator broke it, most often through `renameProperties` or a
self-defending check that a later layer reformatted), when the input to a layer is over
`--max-layer-input` bytes (100 KB) or when a layer's output is over `--max-bytes` (600 KB). A
file that `disableConsoleOutput` silences counts when, run again with the console methods made
read-only so the silencing cannot take hold, it prints what the source prints. Every node run preloads a `document`
on `example.com`, so the domain lock passes and stays testable. A file with debug protection on an
interval never exits, so for those node runs stop after `--hang-timeout` seconds instead of
`--timeout`, and the output must print the same and keep running. Then it runs Untwist in both lanes
with `--report json`, runs node on each output, and looks for leftover machinery: Untwist run again
with `--unsafe` on the output must find no string array, decoder, wrapper, self-defending, debug
protection or console code, and the output must hold none of the protection fingerprints (string
array function, rotator, base64 and rc4 decoders, the `(((.+)+)+)+$` check, `debu`/`gger`,
console method lists, the domain list, call controllers, the global-object getter).

- `--unsafe` passes when the output prints what the source prints and nothing is left.
- The default lane passes when the output prints what the source or the obfuscated file prints and
  nothing is left, or when something is left and a note from a machinery pass names the premise it
  was kept for (code that may hold the global object, code from text, a replaced built-in, the
  outside-code premise, a self-defending check that would hang the reformatted output).
  `summary.json` counts those as `kept`. A bare "decoder machinery kept" or "a reference survives"
  note does not count.

Build Untwist and run the whole sweep inside one run slot:

```sh
CARGO_TARGET_DIR=$HOME/.cache/untwist-targets/ox1 untwist-build-slot cargo build --release -p untwist-cli
UNTWIST_BIN=$HOME/.cache/untwist-targets/ox1/release/untwist untwist-run-slot node tools/obfuscated/sweep.mjs
```

Results go to `~/.cache/untwist-tmp/ox1/run` (`--out DIR`): `plan.json` holds the option domains
and every combination, `results.jsonl` one row per case, `summary.json` the pass rates per lane and
layer count, and `cases/<id>/` keeps the obfuscated file, both outputs and `case.json` for every case
that failed (`--keep` keeps all of them). `--plan` prints the case count and stops, `--filter RE`
runs the cases whose id matches, `--limit N` the first N, `--random N` sets the number of random
combinations (32), `--seed N` the seed (51), `--jobs N` the worker count (10) and
`--layer-stride N` runs the layered chains for one source in N. Node runs get a 96 MB heap and
Untwist `--max-memory 768MiB`, and at most two Untwist runs on inputs over 150 KB go at once
(`--big-jobs`), which keeps the run under the slot's 6 GB. Each case is obfuscated in a fresh node
process, because javascript-obfuscator keeps state between calls in one process, so a case gives the
same file whatever ran before it and `--filter` reproduces it. A full run of the 3,465 cases takes
about 34 minutes with 10 jobs in one run slot and peaks at about 5 GB. `--rejudge DIR` applies the
pass rules again to a finished run's `results.jsonl` and rewrites its `summary.json`.
