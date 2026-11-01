# Known divergences from the native build: WebAssembly

The wasm build (`crates/untwist-wasm`, `wasm32-unknown-unknown`, run by Node through
`harness/run.mjs`) gives the same output bytes as the native CLI, with one exception: it runs on
stacks that cannot grow. Native Untwist parses every input on a 1 GiB stack segment and grows the
stack (`stacker`) wherever the evaluator recurses. On wasm, V8's stack and the module's shadow
stack are fixed, so Untwist admits less and stops some evaluations sooner. Every such limit is a
refusal or a bail, never a different value and never a trap.

## The stacks and the budget

Both stacks were measured under node v22.15.0 by bisecting the depth at which each generated depth
class traps (`depth_classes::ALL`, the three CLI flag sets), at two stack sizes, with V8's
baseline tier only and its optimising tier only. The cost per counted nesting unit is the slope
between the two sizes. Node's flags served only to measure; the build relies on none.

| Stack | Size | Used before the input's recursion | Worst measured per unit | Unit charged (2.3 ×) | Units it admits |
| --- | --- | --- | --- | --- | --- |
| V8's stack (the wasm call chain) | 984 KiB, V8's default | 23,173 B (charged 23 KiB) | 639 B (template literals, parse) | 1,470 B | **661** |
| shadow stack (linear memory) | 1 MiB, the linker's default | 7,618 B (charged 8 KiB) | 576 B (template literals, parse) | 1,325 B | 777 |

`nesting ≤ (stack − entry) / unit − 8`, the smaller of the two: **661 units**. The `− 8` is
`C_P1`. Native admits `min(100,000, room / (2 × UNIT_STACK_BYTES) − 8)` units. The shadow stack
never binds first, so the module keeps the linker's default size; `harness/budget.mjs` checks that
the module's `__stack_pointer` starts where the budget assumes.

Re-measured on `9923575` (fused deobfuscate scans, hooks dispatched by variant), the worst cost is
the same, so the budget stands. Three classes moved: `numplus`, `strplus` and `unary` under the
default flags now cost 160 B per unit, against 152, and `block` traps 2 units earlier.

The shadow stack lies below every static and the heap (stack-first layout). An overflow wraps
below address 0 and traps as `memory access out of bounds`; it cannot reach live data. Each of
the 198 shadow-stack overflows that the measurement provoked, on 1 MiB and 2 MiB stacks, trapped
that way.

Where each stage traps first, on V8's stack, in bytes per unit (the slope between 984 and
1,968 KiB, optimising tier):

| Stage | Cells where it traps first | Bytes per unit |
| --- | --- | --- |
| parse | 42: every bracket, statement and right-recursive class | 101–623 (639 under the baseline tier) |
| deobfuscate's `calls::agnostic::Sites` scan | 12: left-deep chains under the default flags | 247–503 |
| codegen | 7: left-deep chains under the bare flags | 208–408 |
| reflow's scan, and leaves called from the walks | 26 | 143–247 |
| `calls::Collector` | a 3,415-term `+` chain in a decoder body | about 288 |
| semantic, the `PassGroup` walk, the evaluator over the source's nesting | none | below the stage that traps first |

## What the wasm build does differently

| Where | Native | wasm | Note or effect |
| --- | --- | --- | --- |
| an input nesting more than 661 units | parsed (up to 100,000 units) | refused before parse | `this input nests deeper than untwist can parse on its stack; it is passed through unchanged`, exit 3 |
| the unpack re-parse of the final texts | trusts the re-parse premise (`2 × nesting + 8`) | scans each final text against the same budget | a deeper final text: `a final text nests too deeply to parse again for unpacking; the inputs are written without a bundle` |
| a `JSON.parse('…')` literal | inlined when it nests at most the input's nesting + 8 | inlined only when it also nests at most 661 − the input's nesting, so the tree stays within the budget | no note; the output keeps the call, which is node's value |
| a decoded or literal code text (sinks, `Function` bodies) | parsed when it nests at most 512 units, a quarter of its headroom | parsed only when it also nests at most a quarter of 661 − the input's nesting | the text is judged unparsable: its unit stays encoded |
| evaluator call depth | 1,000 | 323 (2.3 × 1,323 B per interpreted call on V8's stack) | the decode stops; its call sites stay encoded |
| evaluator nesting (expressions, calls, `JSON.parse`, `JSON.stringify`) | 100,000 | 1,008 (2.3 × 424 B per level, `JSON.stringify`'s) | the same |

A recursive decoder takes about four nesting levels per call, so on wasm it decodes up to 250
nested calls (native decodes 900 and stops before 999). `JSON.parse` of 999 nested arrays and
`JSON.stringify` of 998 decode on wasm, as on native.

## The counted rows

`wasm-depth` (the gate row) runs:

- the 5b depth record: 35 classes, each at its largest native-admitted depth, one deeper, twice
  the 16 MiB kill depth and 1,000,000;
- every class of `depth_classes::ALL` whose nesting grows (57 of 59), at its largest wasm-admitted
  depth and one deeper;
- fourteen probes: eight for the evaluator, four code texts in a `setTimeout` sink, two
  `JSON.parse` literals.

The depth inputs run under the three flag sets and the probes under `--unsafe --no-unpack`: 764
cells, each compared with native. The whole set runs twice: once on V8's default stack, and once
with only 441 KiB, which is V8's room ÷ 2.3 plus the entry, under the optimising tier. **No cell
traps in either run.** The code-text probes run a third time at 200 KiB, where only the text cap
keeps the stacked texts from trapping. A fourth run appends a statement to the wasm output of nine
cells, among them a cap that fired and an admission refusal; each must then be reported as
`differ`. The module's budget, its shadow stack and the two evaluator caps are compared with the
pinned 661, 1,048,576, 323 and 1,008.

| Cells | Native | wasm |
| --- | --- | --- |
| 171 (57 classes × 3 sets) at the wasm edge | output | the same output |
| 171 one level past the wasm edge (662 units or more) | output | refused (nesting) |
| 12 depth-record cells (`comma`, `stmts`: flat) | output | the same output |
| 165 depth-record cells | refused (nesting) | the same refusal |
| 108 depth-record cells | refused (print size) | refused (nesting): the same text, another note |
| 123 depth-record cells (4,095 units or more) | output | refused (nesting) |
| 3 probes: 200 nested calls, `JSON.parse` 900 deep, `JSON.stringify` 900 deep | decoded | the same output |
| 3 probes: 900 nested calls, `JSON.parse` 8,000 deep, `JSON.stringify` 3,000 deep | decoded | the decode stops, call sites encoded |
| 2 probes: 600 calls of `(n) => n && r(n - 1)`, 300 calls of `r.call(null, n - 1)` | decoded | the decode stops (the call cap), call sites encoded |
| 2 probes: a 141-unit text at a 28-unit site; four nested texts of 61 units each | decoded | the same output |
| 1 probe: a 511-unit text at a 608-unit site | decoded | not decoded (the text is over (661 − 608) / 4) |
| 1 probe: four nested 509-unit texts at a 154-unit site | decoded | not decoded |
| 1 probe: a 201-unit `JSON.parse` literal at a 201-unit site | inlined | the same output |
| 1 probe: a 661-unit literal at a 655-unit site (input 658) | inlined, the tree nests 1,315 units | the call is kept |

Every input refused on wasm where native gives output nests more than the 661-unit budget. The
shallowest is 662 units, and every depth-record one is at 4,095 or more.
`wasm-diff` (the diff sets, the behaviour corpus, `corpus-npm`, game.js and the high preset in
every mode, and the three renames) has no wasm-only refusal: 632 of 632 cases are identical. The
deepest of those inputs is game.js, at 505 units; its final text nests 402.

`harness/diff.mjs` reports a differing case as `refusal` only when the difference is explained by
the limit that fired:

- an admission refusal by the wasm budget is explained when the output is exactly the input passed
  through (the module's own pass-through of the same input), with the single refusal warning and
  exit 3;
- a cap that keeps code at its site (a code text or JSON literal over its cap, an evaluator cap) is
  explained only by a pin: the case's id, the limits that fired and its exact output digest, from
  `UNTWIST_WASM_PINS`. The `wasm-depth` probes are pinned; `wasm-diff` has no pins.

Any other difference, including a wrong value elsewhere in an input whose cap fired, is `differ`.
A final text kept from unpacking is never explained, so it is always `differ`.

Nested code texts are parsed on top of one another: escape analysis parses a sink's text inside the
visit of the text that holds the sink (`escape.rs`), up to four deep. Without a cap of their own,
four stacked 509-unit texts at a 154-unit site trap at 400 KiB (optimising tier) and pass at
441 KiB (510-unit texts at a 148-unit site do the same). The cap therefore mirrors
native's, which gives each text a quarter of the decoded-text headroom: on wasm a text nests at
most a quarter of 661 − the input's nesting, so the site and four stacked texts together stay
within 661 units. With the cap, the stacked probe keeps its unit encoded and runs within 200 KiB.