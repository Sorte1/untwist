# Known divergences from the reference deobfuscator: Evaluator

`untwist-eval` is a fuel-bounded interpreter for obfuscator.io decoder programs, not a JS
engine. The items below are deliberate; every one of them is pinned by
`crates/untwist-eval/tests/divergences.rs` unless its bullet names another file.

- A decode unit may spend **50,000,000 interpreter steps, and 1,000,000,000 under `--unsafe`**
  (`--fuel` sets either). The reference deobfuscator runs the decoder in a sandbox with a ten
  second timeout instead. A rotator whose checksum matches only after thousands of rotations of a
  large array costs the search hundreds of millions of steps: one Krunker 9.2 build needs about
  265 million, some three seconds of work. The default lane gives up on such a search and leaves
  the calls; `--unsafe` finishes it, as the reference deobfuscator does. Pinned by
  `crates/untwist/tests/unsafe_fuel.rs`.
- `f.bind(null).toString()` returns the **target's** source text. V8 returns
  `function () { [native code] }` for a bound function. Self-defending checks that regex the
  source of a bound function therefore see the original body. While decoding, the source text
  of a bound function or a built-in bails instead.
- `Math.random()` **bails** in every context. It used to return a stubbed `0.5`,
  which is a guess about a value no evaluator can know; a decoder that reads randomness is
  now refused. `Object.freeze` bails while decoding (it is modelled outside: see
  the attributes bullet).
- Regexes run over a UTF-8 transcoding of the subject, which matches by code point where JS
  matches code units without the `u` flag. A regular expression run over a subject that holds
  any surrogate code unit therefore **bails**, in every context:
  `'\u{1F600}'.replace(/./g, 'x')` and `'\u{1F600}'.split(/(?:)/)` are refused, not answered.
- `splice` coerces its arguments first and then clamps against the array as it is after
  coercion. `map`, `filter`, `forEach` and `reduce` read the **live** array as the
  spec does, with the length fixed when they start; a `map` whose callback shrinks the array
  (which leaves holes in node's result) bails. `for-of` over an array reads it live too.
- There is no temporal dead zone: a `let`/`const` binding only exists once its declaration
  executes, so a read before it resolves to an outer binding of the same name or bails as an
  unresolved global, where JS throws a `ReferenceError`. **While decoding** this cannot be
  reached: a decode session is refused when its machinery has a function body or block whose
  lexical binding is referenced before its declaration ends (or from a hoisted function), a
  switch whose cases declare lexically, or a lexical loop head that names its own bindings. `for (let …;;)` and `for (let … in/of …)` give each iteration its own
  binding, as JS does.
- Strictness is modelled. An assignment to a name that resolves nowhere throws a
  `ReferenceError` in strict code, after the right-hand side runs, as V8 does; in sloppy code it
  creates the global. Code is strict under a `"use strict"` directive of the script, a function
  or an arrow, inside a class body, or in a program with real module syntax. A decode unit no
  longer runs under one flag for all its functions: the string-array function, each decoder and
  the rotator take the strictness of their own context, and the rotator's checksum also honours
  a directive in the rotator's body. "Resolves nowhere" means nowhere in the evaluator's realm;
  the pipeline refuses any decoder or rotator that writes a name outside its machinery, so a
  decode never depends on what the host's global object holds. Pinned by
  `an_unresolved_assignment_in_strict_code_throws_a_reference_error` and the strict rows of
  `crates/untwist-eval/tests/decode_context.rs`.
- `for-in` takes its key list when it starts and, before each key, asks whether the object
  still has it (own property, then `HasProperty` through the prototype), as V8 does: a key
  deleted before it is reached, or an index that a `pop`, `splice` or shorter `length` removed,
  is skipped; a deleted key the prototype still holds (`toString`) is visited; a key added
  during the loop is not visited; a key deleted and added again keeps its first place.
- **Property attributes**. Every own property carries the writable,
  enumerable and configurable bits, kept in its key and rewritten in place, so a redefinition
  or a freeze keeps the key order. Built-ins install their spec attributes: methods,
  `constructor` and error messages do not enumerate; `Math` and `Number` constants and a
  built-in constructor's `prototype` are fixed; a function's `prototype` is writable but
  neither enumerable nor configurable, so `Object.keys`, `for-in`, `Object.assign` and
  `JSON.stringify` skip it and `delete F.prototype` answers `false`. `Object.keys`, `for-in`,
  `JSON.stringify`, `Object.assign`, `delete` and assignment honour the bits; an assignment
  also honours a read-only property that the prototype chain holds and an object that is not
  extensible; a refused strict write or delete throws a `TypeError`, a sloppy one is ignored.
  `Object.defineProperty` takes data descriptors with the spec's defaults and checks, and
  `Object.freeze`/`Object.isFrozen` model an ordinary plain object. Limits: accessors are not
  modelled, so an accessor in an object literal or a descriptor **bails in every context**;
  `Object.freeze` of anything but an ordinary plain object bails, and so does
  `defineProperty` on a built-in, on an array's `length`, or on an array or `arguments` element
  unless the descriptor is a plain data one (an own value and every attribute true). While
  decoding, `freeze` bails and `defineProperty` keeps that plain-data rule for every target, so
  no decode-time write can meet a read-only key or a non-extensible object (built-ins already
  refuse writes), and the decode path skips the check. The attribute bits live in the map's
  stored key (`StoredKey`, 16 B, so the bucket stays 40 B); the lookup key `PropKey` carries
  none, so member reads and writes move no extra byte. Pinned by
  `crates/untwist-eval/tests/attributes.rs` (node-checked) and `eager_prototype.rs`.
- `new F()` on a function whose `prototype` was overwritten with a primitive replaces it with a
  fresh object: `function F() {} F.prototype = 5; new F(); typeof F.prototype` is `"object"` here
  and `"number"` in node, where `new` only falls back to `Object.prototype` for the instance and
  leaves `F.prototype` alone. Pinned by
  `new_on_a_function_whose_prototype_is_a_primitive_replaces_it_as_the_interpreter_constructs`.
- `Number.prototype.toFixed` formats through Rust, which breaks exact decimal ties to even where
  JS rounds them up, so a tie (`(2.5).toFixed(0)`) **bails** in every context.
  `**`, `Math.pow`, `Math.log` and a radix other than 10 for a fraction or an integer at or
  beyond 2^53 also bail unless the result is exact. `Math.round` is exact.
- Error objects carry only a `message` property: `e.name` is `undefined` and `String(e)` is
  `"[object Error]"` (node: `"TypeError: …"`). The `Error`/`TypeError`/… constructors are not bound as globals at all
  so `new TypeError(…)` bails as an unresolved global. A `typeof` of a name the realm does not
  bind bails **while decoding**, and outside decoding too when node binds it (its global object's
  names plus the CommonJS `require`, `module`, `exports`, `__filename`, `__dirname`); any other
  unbound name is `"undefined"`, as in node.
  **While decoding** an error the evaluator raises has no properties at all, and any read, `in`
  or `hasOwnProperty` that misses on it bails.
- **While decoding**, a read or `in` that misses every property the evaluator models bails when
  node would find the key: any name on a built-in object, constructor, prototype or native, and
  for functions, arrays, regexps, strings and numbers any key on node's own or prototype list
  (`length`, `name`, `arguments`, `caller`, `flatMap`, `lastIndex`, `toLocaleString`, …), plus
  every key on `Object.prototype`. Outside decoding such a read is `undefined`.
- `({}).constructor` is `undefined` outside decoding. `Object.prototype.constructor` is
  deliberately bound to `undefined` so that `({}).constructor('return this')()` cannot reach a
  host `Function`. **While decoding**, a `constructor` read that resolves to that placeholder
  (`[].constructor`, `({}).constructor`) bails, and so does an inherited `constructor` of a
  generator, async or bound function, whose constructor is not `Function`. A user
  function's lazy prototype has its own non-enumerable `constructor`, so an instance's
  `constructor` is the function, as in node.
- `new` on a bound function constructs its target with the bound arguments first, so the
  instance inherits the target's prototype and `instanceof` holds for both.
- A RegExp's `lastIndex` is tracked internally but is not readable or writable from JS;
  `re.lastIndex` is `undefined` outside decoding, and a read or write of it bails while
  decoding.
- `fold::is_pure` means "no side effects", **not** "cannot throw". `null.x` and
  `decodeURIComponent('%')` are both pure and both throw, so a dead-code consumer that deletes
  pure expressions deletes observable throws. (`JSON.parse('x')` happens to be impure here for
  an unrelated reason — a member call is only pure when its receiver expression is itself
  pure, and the identifier `JSON` is not — but do not rely on that.)
- Hostile host objects (`window`, `document`, `navigator`, `process`) are `typeof "object"`
  and hostile host functions (`eval`, `Function`, `setTimeout`, `Date`, `fetch`, …) are
  `typeof "function"`, but every other access — reading a property, writing one, calling or
  constructing — bails with `hostile global <name> reached`. Outside decoding
  `typeof` on the bare identifier still answers where node binds the name (`process`,
  `navigator`, `eval`, `setTimeout`, …) and bails where it does not (`window`, `document`,
  `requestAnimationFrame`, `XMLHttpRequest`); **while decoding** a `typeof` of a host
  object the realm stands in for (`window`, `self`, `global`, `frames`) bails, because its
  answer depends on the host: in a node script `typeof global` is `"object"` and
  `typeof window` is `"undefined"`, in a browser the reverse.
- The constant folder resolves the names `undefined`, `NaN` and `Infinity` (and every name the
  realm binds, such as `String`) to their global values without consulting scope, since
  `untwist-eval`'s `fold_identifier` has no scoping tables. Every caller that can
  hand it a shadowable name checks scope first, through `untwist_core::ast_util::global`: a
  folded identifier must resolve to no binding, sit inside no `with` statement, and the program
  must hold no direct `eval`. `fold-literal-chains` refuses a chain such as
  `"abc".charAt(NaN)` under `function (NaN) {…}`, and `control-flow-object`'s dead-branch scan
  stops treating a branch as dead when its test reads such a name (or a non-parameter local) —
  before the fix, `if (_0x.aB(undefined, 3)) { _0x.cD = … }` under a shadowing `undefined`
  counted the mutation as dead and inlined the stale `cD`. `dead-code` and `evaluate-globals`
  fold only string literals. Both modes.

- **Native shapes.** Every native the evaluator implements declares,
  in the single table `crates/untwist-eval/src/builtins/shape.rs`, the receiver and argument
  kinds it models exactly. A call outside that shape **bails in every context**, decode and
  fold alike. Among the refusals: an `arguments` (or any non-array) receiver of an
  `Array.prototype` native, directly or through `call`/`apply`; a RegExp argument to
  `startsWith`/`includes`/`endsWith`; a non-string receiver of `String.prototype.toString`/
  `valueOf`; a JSON reviver, replacer or numeric/string indent; `RegExp(re)` with a RegExp
  pattern; `atob`/`btoa` with no argument; `Object.assign`/`defineProperty` onto anything
  but an array, plain object or regexp; `Object.create` with anything but a plain-object
  prototype (a function, regexp, array or `arguments` object too) or a second
  argument; `Object(primitive)`, which would box. `cargo xtask diff-eval` (below) exercises
  the table in and out of shape.
- **User code inside a native's coercion.** While decoding, a native whose
  argument or receiver conversion would call a user function (`valueOf`, `toString`,
  `Symbol.toPrimitive`) bails, because most natives here read their array or string at a
  different moment from the spec relative to that call. A shape may declare that it converts
  in spec order; only `String.prototype.split` does (receiver, then limit, then separator).
  Callbacks a native invokes (`map`, `forEach`, `filter`, `reduce`, `replace` with a
  function) are not conversions and stay modelled.
- **Prototypes.** While decoding, `new F()` bails when `F.prototype` is not a
  plain object, so no object the evaluator builds has a function, regexp, array or error in
  its chain.
- Exactly modelled: array-index keys enumerate first in ascending order;
  `` $` ``, `$'` and two-digit `$nn` replacement templates; `match`/`search` compile a string
  pattern; Array `indexOf`/`includes` `fromIndex`; `splice()` with no arguments;
  `Object.prototype.toString` class tags; `hasOwnProperty` on a string primitive; case mapping
  around lone surrogates; `parseInt` beyond 2^128 in radix 10 and power-of-two radices (other
  radices bail there); `Math.max`/`min` order `-0` below `+0` and coerce every argument;
  `Number.MIN_VALUE` is `5e-324`; `RegExp.prototype.toString` escapes its source and orders
  its flags as V8 does.
- Exactly modelled or refused: String `indexOf`/`includes` clamp a negative
  position to 0; `Object` is a constructible function that returns an object argument
  or a new object; `Object.prototype.toString` tags generators, async functions and
  async generators; `parseInt` above 2^53 in a radix other than 10 or a power of two
  bails, since V8 accumulates there in doubles; the source text of a method, getter or
  setter bails, because its span starts after the key V8 includes.
- RegExp case folding: without `u`/`v`, an `i` pattern that names a non-ASCII character, or an
  `i` match over a non-ASCII subject, bails (V8 canonicalizes with toUpperCase, regress folds
  case); `\u{…}` without `u`/`v` bails (Annex B reads it as a quantified `u`). While decoding
  the narrower allowlist below applies on top.
- Functions: a function expression's own name is an immutable binding (a sloppy write is
  ignored, a strict write throws); methods, accessors, arrows, generators and async functions
  have no `prototype` and are not constructible; calling a generator or async function bails.
- The decode-context bails above apply to the whole unit: the string-array function, the
  rotator and every decoder call. A string array whose literal reads an `undefined` that a scope declares (a parameter,
  a `var` in a CommonJS module, a block `let`), or one inside a `with` statement or in a program
  with a direct `eval`, is refused by `find-string-array`, since the evaluator would
  give that entry the real `undefined`. The same rule refuses a simple (non-string-array) array
  that `find-string-array` would otherwise inline: `(function (undefined) { var a = ['x',
  undefined]; … a[1] … })('Y')` used to inline `void 0` and print `xundefined` instead of `xY`.
  The array is now kept whole, with its own note, in both modes.

### Decode surface: default-deny

Decode context models only what decoders need: it admits an explicit allowlist; everything
else bails while decoding. Where a pinned input needed a shape outside the list, that one
shape is modelled exactly and added to the list.

- **Regular expressions** (`crates/untwist-eval/src/builtins/regexp_syntax.rs`). Admitted:
  literal characters; `.`; classes and ranges of literal characters (non-empty, a `-` only at
  either end, low ≤ high); `\d \D \s \S \w \W`; the escapes `\n \r \t \f \v`, `\xHH`, `\uHHHH`,
  and `\` before a syntax character or `/`; quantifiers `* + ? {n} {n,} {n,m}` (bounds up to
  1000) with an optional lazy `?`, after an atom only; capturing groups and `(?:…)`;
  alternation; `^ $ \b \B`; backreferences `\1…` to a group that exists; flags `g i m s u v`,
  except `u` and `v` together, which V8 rejects with a SyntaxError (the
  pair is rejected in every mode: decoding refuses, and evaluation outside a decode throws).
  Modelled exactly because a pinned input needs it: without `u`/`v`, a `{` that does not begin
  a valid quantifier and a lone `}` or `]` are literals (Annex B; obfuscator.io's
  self-defending pattern `\w+ *\(\) *{\w+ *['|"].+['|"];? *}` in the high-preset pin), and `\`
  before any character that is not ASCII alphanumeric or `_` is that character (Annex B
  identity escape; the regexp toString row).
  Bails: flags `y` and `d`; lookahead and lookbehind; named groups; modifiers such as
  `(?i:…)`; `\p{…}`/`\P{…}`; `\c`, `\0`, `\k` and every other letter escape; characters above
  U+FFFF or surrogates; any non-ASCII character under `u`, `v` or `i`; any class under `v`;
  under `i`, a class holding a letter or a class escape, a negated class, and a backreference.
  Under `i`, `u` or `v` a non-ASCII subject bails.
  Matching runs under a step budget: the backtracking engine (regress 0.12's
  PikeVM backend, vendored in `vendor/regress` with one addition, `find_with_step_budget`)
  counts every interpreter step and stops a search after 2^22 steps; every search charges a
  quarter of its steps to the session's fuel. An exhausted search bails with `Bail::Fuel`, so a
  catastrophic pattern such as `/^(a+)+b$/` over 43 `a`s refuses the decode in about a second
  instead of hanging (V8 backtracks exponentially on it as well). Fold never evaluates regexp calls.
- **Case mapping.** `toLowerCase`/`toUpperCase` map ASCII only; any non-ASCII unit in the
  receiver bails, **in every context** (decode and fold): Final_Sigma depends on neighbouring
  characters' Unicode properties, which differ between Rust 1.98 (Unicode 17) and node 22.15
  (Unicode 16). `toLocale*` bail as unmodelled keys.
- **Operator keys.** A key that an operator converts (a member read or write, compound and
  logical assignment, update, `delete`, `in`, a computed key in an object literal) bails when
  it is not a primitive, so V8's order and count of conversions never matter. `in` with a
  primitive on the right bails before converting its key. A `null`/`undefined` base with a
  primitive key throws after the key expression runs, as V8 does; a write to it bails.
- **Property writes** bail when the evaluator does not model the property's attributes or
  accessors: a function's `name`, `length`, `arguments`, `caller` or `constructor`; any key of
  a bound function, a native, a regexp (`source`, `flags`, `global`, … are accessors) or an
  error; an `arguments` object's `length` or `callee` (non-enumerable own keys); any intrinsic
  object or prototype. New keys on an `arguments` object are ordinary writes and stay admitted
  (obfuscator.io's decoder caches into one; the high-preset pin).
- **Prototypes.** `Object.create` and `new F` bail on a prototype that is an intrinsic other than
  `Object.prototype` (`Object.create(Math)` is tagged `"Math"` through `@@toStringTag`), on top
  of the plain-object rule.
- **Function objects.** `Object.prototype.toString` of a bound function is tagged by its final
  target, as V8 does (a bound generator is a `GeneratorFunction`). The `constructor`
  placeholder and generator, async and bound constructors bail (see above).
- **Annex B.** A function declaration in a block, an `if` branch or a labelled statement in
  sloppy code bails, in both modes (node also binds it in the enclosing function); one in a `switch` case or a `catch` body bails
  in strict and sloppy code alike, since the evaluator never hoisted it into that scope.
- **Host exceptions.** `atob`/`btoa` on invalid input bail: node throws a `DOMException`, which
  the evaluator does not model.
- **Dead zones.** `eval_script_decoding` runs the same static dead-zone scan as a decode session.

### Differential check: `cargo xtask diff-eval`

`cargo xtask diff-eval [--seed N] [--cases N]` (defaults: seed 1, 20,000 cases) draws calls
from the native shape table, with receivers and arguments inside and outside each declared
shape (primitives including `-0`, `NaN`, 2^53+1, non-ASCII and lone surrogates; arrays and
holes; objects with `valueOf`/`toString`; mutating callbacks; regexps; mapped `arguments`).
Each case is evaluated by Untwist in decode context and, when its operands are literals, by the
fold evaluator, and by node; results are compared structurally. A bail passes; an answer that
differs from node fails the run. It needs node, so it is not part of `cargo test`.

The generator also draws argument objects whose `valueOf`/`toString`
mutate the receiver, generator, async, method and bound functions, objects with a function,
regexp or array prototype, an error object, and Unicode 17 and astral case-mapping text; it
calls through `apply`, `bind`, `Reflect.apply` and as a `map` callback with a receiver, and
4 % of cases are `typeof` of a global name. A fold case renders `(receiver).name(...)` only
when the receiver is the native's natural kind, so its label names the native it calls. The
address-keyed root table is cleared after each decoding evaluation, so a later fold object at
a reused address no longer renders as an intrinsic, and runs are deterministic. **The gate is
seeds 1–10 at 200,000 cases each with no answer that differs from node**.

Diff-eval is statement-level and does not flatten:
- 40 % of cases are statement templates: binary and unary operators, member reads, plain,
  compound and logical assignment, update, `in`/`delete`, `typeof` of host names, for-in with
  mutation, for-of, let-loop closures, labels, switch, dead zones, Annex B block functions,
  try/catch with throwers and observers, and hand-picked samples. Keys and
  values include objects that log their conversions.
- node evaluates each case through a direct `eval` inside the CommonJS module of a real
  script, so `global`, `process`, `require` and the other host names exist as they do for
  `node file`. A guard restores any key a case adds to an intrinsic or to `globalThis`, and
  any intrinsic property whose value or getter a case replaced, and marks that case as a
  difference, so state never leaks into later cases. Mutating natives (`Object.assign`,
  `defineProperty`, `freeze`, `setPrototypeOf`, the Array mutators) draw only fresh samples,
  since a non-configurable key defined on an intrinsic cannot be restored.
- Answers render functions with their class tag and source (methods as `M`, bound functions
  as `B`, natives by label), objects with their prototype and extra own keys (an array's
  `index`/`input`), errors with their prototype (so the error class is compared) and
  intrinsics by name. An answer the renderer cannot express counts as a difference, not a
  bail. A thrown error's `message` is not compared: decode context raises errors without one,
  and any read of `message`, `name` or `String(e)` on them bails, which the try/catch
  templates exercise.
- Each seed answers about 88,700 of 200,000 cases in decode context and folds about 26,100.

15 % of the non-statement cases are regular expressions generated from
the allowlist grammar itself (`xtask/src/diff_eval/regexps.rs`): literals including non-ASCII,
every admitted escape, classes and ranges, `.`, capturing and non-capturing groups,
alternation, every quantifier form with lazy markers, anchors and word boundaries, and
backreferences. 15 % of patterns get a near-miss just outside the grammar spliced in
(lookaround, named groups, `\p`, `\c`, `\0`, modifiers, inverted or oversized bounds, Annex B
braces, astral characters, unbalanced syntax). Flags are random subsets of `gimsuv` in random
order, 10 % of them invalid (`uv`, `vu`, `guv`, duplicates, `y`, `d`, `x`); subjects mix ASCII,
line terminators and case-folding traps (`ß`, `Σ`, the Kelvin sign). Each case runs `test`,
`exec`, `replace` (string and callback), `split`, `match`, `search`, `toString`, `source`,
`flags`, `lastIndex` or a global `exec` loop. Without the `u`+`v` rejection, seed 1 × 20,000 reports 9
differences, every one a `u`+`v` flag string.

### Caps that bail rather than diverge

These are not silent divergences — each one produces a `Bail` naming itself — but they bound
what the evaluator will accept: 5·10⁷ steps, `max(4·array length, 100,000)` iterations per
loop, 1,000 call frames, 100,000 nesting levels, 16 MiB strings, 1,048,576 array elements,
256 scope-chain hops, 64 bound-function hops, 64 prototype-chain hops, and a 4 KiB
regular-expression pattern, and 2^22 steps per regular-expression search.


- **A function declared in a sloppy block bails (no Annex B.3.3 hoisting).** In sloppy code an
  engine also assigns a block-level function declaration to the enclosing function's `var` binding
  when the block runs, so `var g = function () { return 1; }; { function g() { return 2; } } g()`
  returns 2 in node. The interpreter does not model that binding, so the block bails in both
  modes. Strict code keeps the block
  scope, as node does. Obfuscator.io's decoders and rotators declare no function in a block, so
  no decode depends on it. Pinned by `crates/untwist-eval/tests/divergences.rs`
  `a_function_declared_in_a_sloppy_block_bails`.

- **The Unicode-17 refusal rows depend on node's ICU, not on Untwist.** The evaluator's case
  mapping and `\p{…}` tables are Rust 1.98's (Unicode 17) and Untwist refuses the characters the
  older tables would mishandle. The rows that pin those refusals recorded what node 22.15 prints,
  whose ICU predates Unicode 17. Node 22.23 accepts the construct and prints something else.
  When node's output on a row's input differs from the recorded text, the row prints a note and
  checks that node prints the same on Untwist's output as on the input, so the refusal stays
  pinned under any node. Pinned by `check_with` in `crates/untwist/tests/refused_rows/mod.rs`.

- **`instanceof` and `Object.getPrototypeOf` follow the built-in prototypes to `Object.prototype`.** `Array.prototype`,
  `Function.prototype` and `RegExp.prototype` have `Object.prototype` as their `[[Prototype]]`. An
  object whose own link the evaluator leaves unset takes the prototype of its kind: a function,
  native or bound plain function `Function.prototype`, an array `Array.prototype`, a regular
  expression `RegExp.prototype`, an `arguments` object `Object.prototype`. So `[] instanceof Object`,
  `/a/ instanceof Object`, `(function () {}) instanceof Object` and
  `Object.getPrototypeOf(Object.getPrototypeOf([])) === Object.prototype` are `true`, as in node, in
  every mode. A plain object or an error whose link is unset (a `JSON.parse` result, a thrown
  error, a generator or async function) still refuses with "a prototype the evaluator leaves
  implicit": its prototype is not modelled. In a decoder, a bare `Object` on the right of
  `instanceof`, and any answer that crosses a built-in prototype's own link, hold only when the
  program provably cannot reach the built-ins: every property write, delete
  and `assign` target has a base that is a literal or a binding only ever given fresh literals or
  functions (never a parameter, `catch` binding or `for`-`in`/`of` variable); the program names no
  `Function`, `eval`, `Reflect`, `require`, `module`, `exports`, `process`, `vm`, timer, worker,
  global object, `import()`, `with` or sloppy `this`, no `__proto__`, `__define*__`/`__lookup*__`,
  `setPrototypeOf`, `defineProperty`/`defineProperties`,
  `getOwnPropertyDescriptor`/`getOwnPropertyDescriptors` or `constructor` key, reads, calls or destructures no property
  outside the unit's own machinery by a key that is neither a static string (a literal, or a
  template literal with no expressions) nor proven numeric (a number, or a binding written only
  with arithmetic), a decoder-call key counting as dynamic, applies `.call`/`.apply`/`.bind` only to a proven binding or literal, and writes neither `Object`, `Object.prototype` nor `@@hasInstance` on `Object`.
  Every oxc node kind that names a property goes through this one key check
  (`proof/property_reads.rs`, an exhaustive match per enum, so a new kind fails to compile):
  member access, static, computed and private; optional chains; binding and assignment patterns
  with static, computed or shorthand keys, in declarations, parameters, `catch` bindings, rest
  elements, defaults and nested patterns; the left operand of `in`; the member reads under
  `super`, a tagged template's tag and JSX member names. So a blocked key such as `constructor`
  is refused in a destructuring pattern or an `in` test too. Object-literal and class members
  (fields, methods, getters, setters, accessors) define own properties of a fresh object and are
  not reads, except the literal `__proto__:` key, which sets a prototype and blocks. `import.meta`,
  `import()`, a static `import`, `with` and TypeScript-only nodes block. Spread, `for`-`in`,
  `for`-`of`, array patterns, static blocks and `new.target` name no key. Otherwise the unit is
  refused as a patched built-in. Pinned by `crates/untwist-eval/tests/interp_member.rs`,
  `crates/untwist-deobfuscate/tests/instanceof_object.rs` and the `builtin-reach-` and `*-instanceof-object` behaviour fixtures.
