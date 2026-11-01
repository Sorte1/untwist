# Decoder shapes

How the evaluator in `untwist-eval` instantiates, rotates and calls obfuscator.io string-array
decoders, and which shapes it covers.

## Benchmarks

`cargo bench -p untwist-eval --bench decode -- --warm-up-time 1 --measurement-time 5` has three
groups:

- `decode/*`: parse, instantiate and rotate. The bench passes an empty `calls` slice, so per-call
  decoding is not included.
- `rotate/*`: parses the fixture once, outside `b.iter()`, and times `instantiate` plus
  `apply_rotation` without the `Evaluator::decode` wrapper. `instantiate` stays inside the loop
  on purpose: `apply_rotation` rotates the session's array in place and spends the session's
  budget, so a session hoisted out of `b.iter()` would match at attempt 0 on the second iteration
  and then bail on an exhausted budget.
- `session_element/20k`: 10,000 `Session::array_first()` calls on a 20,000-element array, with the
  parse and the instantiate hoisted out of the loop, so the row measures the indexed reads alone.

On both obfuscator.io fixtures the rotation search dominates. It is Θ(array length × one decode)
and cannot be cached across candidates: each attempt rotates by one and decodes a different slot,
and the templates' own memo is keyed on `index + array[0]`, which changes with every rotation. A
memo of the whole rotation, keyed on the unit, removes the repeated searches a full pipeline run
would otherwise make.

## The rotation prelude

The per-attempt scope and prelude are built once per search when the prelude performs no computed
member read at evaluation position (`Checksum::prelude_reads_array`). That is the faithful model:
the real rotator runs its prelude once and repeats only its `while` body, all in one shared scope,
so one scope, one prelude evaluation and writes from one attempt visible to the next match the
program being decoded. `prelude_reads_array == true` selects a fallback that rebuilds the scope per
attempt; it exists for preludes whose value depends on the rotation, not as a soundness gate.

`prelude_reads_array` looks only for a computed member read (`f[0]`) at evaluation position. Three
shapes it classifies as hoistable although they do read or mutate the array are known and
unhandled: a static-member read (`f.at(0)`, `f.length`), a call into a helper that indexes the
array, and a static-member mutation (`f.push(f.shift())`) in the prelude rather than the loop body.
No obfuscator.io preset emits any of them; `crates/untwist-eval/tests/rotation_hoist.rs` pins that
the high preset stays on the hoisted path, so a classifier change that moved it onto the fallback
fails rather than passing silently.

## Step budget

`checksum_matches` evaluates the checksum expression's top-level `+` terms left to right and stops
at the first term that throws or makes the running sum `NaN` (the early exit only fires when the
expected value is a number, where a `NaN` accumulator can never compare equal, so it changes no
result). Wrong rotations usually fail in the first term, so the search does not pay for every term
of every attempt.

Measured on `obfuscator.io-high.js`'s rotation search (338 rotations, binary search over the budget
until `apply_rotation` returns `Some(338)`):

| | steps needed | wall clock (release, `instantiate` + `apply_rotation`) |
|---|---|---|
| without the term-wise early exit | 104,772,949 | 1.978 s |
| with it | 21,392,821 | 0.417 s |

The early exit skips *evaluating* the remaining terms, not just summing them. Each term of an
obfuscator.io checksum calls the decoder, so on a failed attempt the later calls do not happen: a
decoder that fills a memo as a side effect of being called has fewer entries after a failed
attempt. The search outcome is unchanged (the attempt could not match), and `find_rotation`
re-evaluates from the first term on every attempt, so nothing depends on the skipped calls. A
decoder shape that needs the whole sum evaluated for its side effects would have to opt out of the
early exit.

`STEP_BUDGET` is 5·10⁷, the smallest round value at or above twice the measured need
(2 × 21.4 M = 42.8 M). The live constant is in `crates/untwist-eval/src/budget.rs`.
`tests/fixtures_reference.rs` runs the high-preset rotation on `Budget::default()`, so the suite
proves the default budget suffices.

## Covered shapes

| Shape | Covered by | Needs |
|---|---|---|
| self-reassigning string-array function | every fixture; corpus, all | closures capturing the env by reference |
| `none` decoder with index shift | `obfuscator.io.js`; corpus `none-*` | sequence expressions, self-reassignment |
| `base64` decoder, inline atob polyfill | `obfuscator.io-multi-encoders.js`; corpus `base64-*` | `charAt`/`indexOf`/`fromCharCode`/`decodeURIComponent`, `~` on mixed types |
| `base64` decoder, `arguments`-as-cache | `obfuscator.io-multi-encoders.js` | aliasable `arguments` object |
| `rc4` decoder with a per-call key | `obfuscator.io-multi-encoders.js`; corpus `rc4-*` | UTF-16 XOR, `charCodeAt`, array swap |
| rotator with a checksum expression | `obfuscator.io.js` and friends | `checksum_from_rotator` + brute force |
| rotator with `!iife` and `for (; [];)` | `obfuscator.io-rotator-unary.js` | IIFE fallback under the loop cap |
| rotator with a non-checksum loop | corpus `*-rot1-*`, `*-rot5-*` | IIFE fallback |
| function wrappers, swapped parameters | `obfuscator.io-function-wrapper.js` | hoisted inner function declarations, closures |
| calls-transform object indirection | `obfuscator.io-calls-transform.js`; corpus `*-ct` | member reads on local objects |
| high preset `StatesClass` self-check | `obfuscator.io-high.js`; `robustness.rs` | `new`, prototype dispatch, `RegExp`, `Function.prototype.toString` |
| `void 0` array entries | `obfuscator.io-string-array-undefined.js` | decode-error marker path |
| out-of-range and negative indexes | `obfuscator.io.js` | `undefined` on OOB read |

`Session::element` distinguishes the two ways an index can fail to name a string: an index at or
past the array's length is `None`, while a hole inside a sparse array literal is a real
`Some(Value::Undefined)` slot that `decoded_to_expression` materialises as `void 0`.
`crates/untwist-eval/tests/session_elements.rs` pins both.

## Priming

`InterpEvaluator::instantiate` primes each decoder with one call before rotation, so the
self-reassigning `b = function (f, g) { … }` body installs itself and the decoder value read back
out of the environment is the real one. The priming call passes `undefined` for both arguments,
never `0`.

The reason is memoisation. Several obfuscator.io templates cache decoded strings, and the cache key
is derived from the index argument. obfuscator.io's own templates key on `f + e[0]`, but a template
keyed on the index alone must not break: priming with `0` before rotation would cache the
pre-rotation decode of index 0, and a later real call for index 0 would return that stale string
with no note and no bail. `undefined` coerces to `NaN`, which cannot collide with any real index,
so the priming call can only fill an entry no decoder call will read.

Decoders that throw on `undefined` (RC4 with an undefined key, for instance) are expected: `prime`
turns the throw into the note `decoder <name> threw while priming; continuing` and keeps the
instantiated decoder.

## Session contract

`InterpEvaluator::decode` is the one-shot path. A step-wise driver (`instantiate` →
`apply_rotation` → `eval_call` per site) relies on this contract, which `decode` hides:

- **`DecoderProgram.rotator` must be the IIFE itself**, not the statement that contains it. Real
  bundles wrap the rotator in a sequence expression (`(function (a, b) { … })(c, 0x1234), d(…)`),
  and the caller unwraps the first element of that sequence before building the
  `DecoderProgram`. A caller that passes the whole sequence gets `None` from
  `checksum_from_rotator` and falls back to interpreting the rotator, which is slower and can fail
  on shapes the checksum search handles.
- **Rotator shapes `checksum_from_rotator` accepts**: a plain call expression, a call wrapped in `!`
  (`!function (a, b) { … }(c, 0x1234)`, the `obfuscator.io-rotator-unary.js` shape), and a
  parenthesised call. Anything else returns `None` and the caller falls back to `run_rotator`.
- **`eval_call` evaluates a call site's arguments in the session's global environment**, not at the
  call site's own scope. The caller must hand it closed expressions (literals, or the object-member
  indirection the calls-transform fixtures use once it has been resolved), never an expression that
  reads a local binding.
- **`Session::budget()`** returns the live `Budget` (`steps_used()` and `steps_left()`), so a
  step-wise driver can report fuel without going through `decode`.
- **`Session::notes()`** returns the session's own notes and the interpreter's (for example
  `Math.random stubbed`) merged; `drain_notes()` takes them both, leaving the session empty, which
  is what a driver that reports per site should use.
- **`Session::element(index)`** is the indexed read: it indexes the dense storage and clones one
  `Value`, returning `None` for an out-of-range index or a non-array. `Session::array_first()` is
  `element(0)`. `Session::elements()` is a full snapshot and copies; use it only when the whole
  array is wanted.
