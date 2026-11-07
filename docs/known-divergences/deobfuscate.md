# Known divergences from the reference deobfuscator: Deobfuscate

- **`decode_error` is a diagnostic, not a comment.** The reference deobfuscator marks a call site that
  did not decode to a string with an inline `decode_error` comment. Untwist emits a
  `Cx::note` carrying `untwist:decode_error` and the call site's span instead, because
  `oxc_ast::Comment` has no text field — it carries only a `Span`, and `oxc_codegen` prints
  `comment.span.source_text(source_text)` (`oxc_codegen-0.148.0/src/comment.rs:413-425`) — so a
  synthesized comment can only ever print a slice of the *input*. The substituted value is
  identical; only the reporting channel differs, and `xtask diff-deobfuscate` normalises with
  comments disabled, so this is invisible to the differential.
- **A decoder wrapper whose call sites were not all rewritten is kept.** The reference deobfuscator's
  `inlineFunctionAliases` calls `decl.remove()` unconditionally
  (`src/ast-utils/inline.ts:229`), so a call site its `inlineFunctionCall`
  refused leaves a dangling reference to a function that no longer exists. Untwist counts the
  rewrites against the wrapper symbol's resolved-reference count
  (`cx.scoping().get_resolved_reference_ids(symbol).len()`) and removes the declaration only when
  the two are equal — which is still `0 == 0` for a wrapper nothing calls, so every the reference deobfuscator
  fixture behaves identically — and otherwise keeps the declaration and emits a note naming the
  counts.
- **Only `dead-code` is `Safety::Unsafe`.** The reference deobfuscator tags `deobfuscate`,
  `inline-decoder-wrappers`, `inline-decoded-strings` and `dead-code` as unsafe, but its
  `applyTransform`/`applyTransformAsync` never filter on the tag — there, the tag is
  documentation. Untwist's `PassGroup::new` *does* filter
  (`crates/untwist-core/src/pass/group.rs:18`), so tagging the decoding passes `Unsafe` would
  silently disable deobfuscation in the default configuration. Every pass in the deobfuscate
  stage is therefore `Safety::Safe`, and `cargo xtask diff-deobfuscate` runs Untwist with
  `--unsafe` so `dead-code` and `template-literals` participate the way the reference deobfuscator applies them
  unconditionally.
- **An alias declaration is only removed when its shape is unambiguous**. Untwist
  removes a `var alias = decoder` declaration only when the alias symbol has no redeclaration,
  is declared exactly once by a single-declarator variable declaration, and that declaration is
  a direct member of a statement list (program body, block, function body, switch-case
  consequent, static block); any other shape is left in place with the note
  `the alias is not declared exactly once by a single-declarator declaration in a statement
  list, so it was left in place`. The reference deobfuscator's `inline.ts` removes the
  declaration unconditionally, which can drop a sibling declarator or a loop head. Effect: an
  alias the reference deobfuscator inlines that Untwist keeps — never invalid output.
- **An alias that is written again is never inlined**. The declarator form requires
  `is_constant_binding` (no other write to the symbol); the assignment form requires an
  uninitialised declarator plus exactly one write, the aliasing assignment itself. A second
  write, or a declarator that already has an initialiser, refuses the alias with `NOT_CONSTANT`,
  `EXTRA_WRITES` or `ALREADY_INITIALISED`. The reference deobfuscator inlines unconditionally and can miscompile
  the same input, substituting a value the binding no longer holds. Effect: Untwist keeps such
  an alias and reports it.
- **A self-assignment of the decoder does not count as a write**. `a = a`, in
  statement or expression form, is excluded from the one-write limit that gates the assignment
  form above: the statement form is removed outright and the expression form is a no-op recorded
  by the note `a self-assignment of the decoder was left in place`.
  The reference deobfuscator has no such carve-out. Effect: Untwist inlines aliases on obfuscator output that
  carries a self-assignment, where a naive write count would refuse them.
- **The wrapper-removal gate counts resolved references, not collected call sites**. `settle_wrapper` compares the rewrites against
  `scoping().get_resolved_reference_ids(symbol).len()` for the current iteration, so a reference
  the call collector never saw still blocks removal; a wrapper whose returned call holds a
  function, arrow or class is refused up front (`WRAPPER_NESTED_FUNCTION`), and a wrapper with a
  deferred call site is reported as `WRAPPER_DEFERRED` and retried next round. The reference deobfuscator removes
  after rewriting the sites it happened to visit. Effect: a wrapper kept for one more iteration,
  never a dangling reference.
- **A spread argument refuses wrapper inlining**.
  `untwist_core::ast_util::inline::inline_function_call` returns `None` as soon as any argument
  is a `SpreadElement`, because the positional parameter mapping is undefined there (the
  rest-forwarding `f(...args)` branch is unaffected); the wrapper pass then leaves the call site
  alone with a note and keeps the wrapper. The reference deobfuscator substitutes positionally regardless. Effect:
  `alias(...xs)` stays as written — a shape obfuscator.io does not emit.
- **An external variable is inlined only when the assignment that reaches the reference is
  structurally decidable**. The assigned-variable path in
  `inline-decoded-strings` requires the literal assignment statement and the declaration
  statement to be direct members of the *same* statement list (the obfuscator's
  `var x; x = 0xd6;` shape), refusing anything else with
  `ASSIGNMENT_NOT_BESIDE_THE_DECLARATION`; and for each reference it requires the reference's own
  enclosing statement to be a member of that same list, at a greater list index, with no
  function, arrow or class boundary crossed on the way out
  (`init_order::InitOrder::slot_of_reference`). The value substituted is the assignment with the
  greatest list index below the reference's, so a reference that sits between two assignments
  reads the one in effect. A reference inside any function body is refused with
  `REFERENCE_NOT_DOMINATED`, because a closure can run at a time source order does not predict:
  the reference deobfuscator's `getNearestAssignment` (`ast-utils/inline.ts:45-47`) picks the nearest *textually*
  preceding assignment and so rewrites `var k; k = 1; function g() { return d(k); } k = 0;` to
  read `1` where the program reads `0`. Effect: Untwist leaves a decodable site behind on a
  bundle that assigns around a function; the reference deobfuscator miscompiles it.
- **An optional call to a decoder is decoded; the reference deobfuscator ignores it**. `collect_calls`
  accepts a `ChainExpression` whose element is a call to a stamped decoder (`__DECODE_0__?.(0x1)`)
  and substitutes the whole chain expression, because a stamped decoder is a hoisted function and
  `d?.(x)` is therefore equivalent to `d(x)`. The reference deobfuscator's matcher only sees plain `CallExpression`s
  and leaves optional calls undecoded. Effect: Untwist decodes strictly more sites than the reference deobfuscator
  here; the substituted chain is replaced whole so the output re-parses cleanly.
- **Every gave-up path reports itself**. Wherever a deobfuscate pass declines to
  act — an unusable alias, a refused call site, a kept wrapper, machinery that is not a statement
  list member — it emits a `Cx::note` naming the pass and the span before moving on, rather than
  silently skipping the way the reference deobfuscator does. The notes are diagnostics only: `xtask::diff::normalize`
  reparses and prints both sides, so none of them reaches the differential.
- **A bare template-literal string array element is accepted and inlined as a string literal.**
  the reference deobfuscator PR #195 widens `find-string-array`'s element matcher to `m.templateLiteral()`; Untwist
  accepts a template literal with no expressions and exactly one quasi, and Pattern B inlines it as
  an `Expression::StringLiteral` rather than as a clone of the template literal. The reference deobfuscator's PR keeps
  the template literal; Untwist normalises because every downstream matcher in `untwist-deobfuscate`
  and `untwist-unminify` keys on `Expression::StringLiteral`, and ``console[`log`](…)`` would
  silently defeat them. The value is unchanged. Narrower than the PR in two ways: an interpolated
  template (`` `a${b}c` ``) is refused because its element is not a compile-time constant, and an
  element whose quasi carries `lone_surrogates` is refused because `value.cooked` is then a
  `\u{FFFD}`-escaped encoding, not the real text.
- **A plain alias of a string array is resolved before the array is inlined** (reference deobfuscator issue
  #230, shapes 1 and 2). `var arr = ['log', 'hi']; var arr2 = arr;` makes the array binding
  non-readonly: `var arr2 = arr` is `arr` alone in a value position, which `ast_util::readonly`
  classifies as opaque, so `inline_simple_arrays` refuses the whole array. The reference deobfuscator has the
  identical bug and the identical cause. `find-string-array` therefore runs a third phase,
  `resolve_array_aliases`, between `stamp_wrapped_array` and `inline_simple_arrays`, which renames
  the alias to the root array's name and drops its declaration so `scan_object_uses` sees one
  binding. It ships with exactly these guards, and every one of them refuses by
  leaving the program untouched:
  - the **root** array's binding is a constant binding, has no redeclaration, and is never written
    through (`binding_is_never_rewritten`), so an array that is reassigned, redeclared, rebound
    inside a function, or mutated by element is never aliased away;
  - **every link** is checked for position, not just the last one: an alias's
    *immediate* source - the root array's statement, or the intermediate alias's own declaration -
    is a direct member of the *same* statement list as the alias and precedes it by list index, and
    the root's declaration must likewise precede the alias that finally reads it. The comparison is
    a structural list index, never a span comparison. So `var c = b; var b = arr;` is
    refused at the `c` link and keeps the TDZ or `undefined` read the original performed;
  - the **alias declaration** is a single-declarator variable declaration that is a direct member
    of the statement list which is the body of the scope *owning* the alias symbol (program body,
    function or arrow body, block, class static block). A `var` inside a nested block, a brace-less
    `if` consequent, or a switch case is therefore refused, because removing it would make a
    conditional binding unconditional;
  - the **alias binding** is a constant binding, has no redeclaration, is never written through,
    and has no opaque use: every reference to it is an index read whose result is used as a value
    (never called, used as a base, iterated or destructured), or there are none left, so
    an alias that mutates the array (`var g = arr; g.reverse()`) is never renamed away;
  - the **root array's name** is the only symbol carrying that name anywhere in the program, so the
    rename cannot land on a binding that shadows the array in any scope that references the alias;
    otherwise the alias is left in place with a note.
  Alias-of-alias chains settle over the stage's fixpoint, one link per iteration, and a root
  propagates through an intermediate alias only when that intermediate is stable by the same test as
  the root **and** is declared earlier in the same statement list than the alias that reads it.
  Because each link's source must precede it in the same statement list, the worklist
  reaches its fixpoint in declaration order, so it runs `while progressed` with no round bound.
  Every reference to an alias must also be provably unable to run before the alias declaration has
  executed (`init_order`), or the alias is left in place with a note. A root an alias
  was renamed into is not inlined in the same round: the renamed references still resolve to the
  alias's symbol in that round's scoping, so the root is inlined one round later, on rebuilt
  scoping, and never loses its declaration while a renamed read remains. Effect: Untwist decodes a shape the reference deobfuscator leaves obfuscated, and
  refuses several shapes the reference deobfuscator's own version of this fix would miscompile. Shape (4), the
  decoder-function alias, is separately covered by `inline-decoder-wrappers`.
- **A file holding several independently obfuscated string-array units is decoded in full**
  (reference deobfuscator issue #70). The reference deobfuscator's `findStringArray` calls `path.stop()` after the first array, so
  every later unit is reported absent and the run prints `String Array: no`; Untwist's
  `find-string-array` stamps **every** match in a single application, numbering the units in source
  order, and each unit then gets its own decoders, its own rotator, its own evaluator run and its
  own removal gate, so a whole multi-unit file is consumed in one working round. There is no
  `additional string array ignored` note — nothing is ignored — and `DEOBFUSCATE_MAX_ROUNDS` (64)
  is unrelated to how many units a file holds; it stays a non-termination guard.
  Because the gate is per unit, a unit whose call sites do not all decode keeps its own machinery
  with the `decoder machinery kept` note while every other unit still loses its own.
  `fixtures/deobfuscate-gaps/two-string-array-units.js` pins the end-to-end outcome that both units
  decode, in `report.iterations == 2` (one working round, then the round that confirms it).

- **A decoder wrapper is inlined only when its body reads nothing but its own parameters**. The reference deobfuscator's `inlineFunctionCall` (`ast-utils/inline.ts:128-163`) clones the
  wrapper's returned expression to the call site and substitutes parameters by name, so any other
  identifier in that body re-resolves in the call site's scope, and `this`, `arguments`,
  `new.target` and `super` change meaning outright. Untwist accepts a wrapper only when every
  identifier reference in the returned expression is one of its own parameters or a stamped
  machinery name, and refuses the rest with a note; `untwist_core::ast_util::inline_function_call`
  refuses a body reading `this`, `super`, `new.target`, `arguments` or `import.meta` on its own so
  the shared utility is safe for all four callers. Effect: on `function w(x, y) { return d(x + s,
  y); }` inlined into a caller that shadows `s`, the reference deobfuscator prints a different string (or throws);
  Untwist keeps the wrapper.
- **A wrapper call site is inlined only when every argument can be duplicated, dropped and
  reordered**. Name-based substitution evaluates an argument once per occurrence of
  its parameter in the body, not at all when the parameter is unused, and in the body's order
  rather than the call's. Untwist therefore requires each argument to be an identifier, a
  numeric/string/boolean/null literal, `void 0`, a unary `-`/`+`/`!`/`~` of one of those, a binary
  expression of those, or a call to a stamped decoder; anything else -- `w(i++, 1)`, `w(f(), 1)`,
  a member read, an assignment -- keeps that call site with a note, and the wrapper with it. A
  site whose arguments still contain an unresolved wrapper call is deferred instead, so nested
  wrappers keep flattening across iterations. The reference deobfuscator substitutes regardless, so `w(i++, 0)`
  leaves `i` at 2 where the program leaves it at 1. Effect: Untwist decodes fewer sites on
  hand-written inputs; obfuscator.io passes literals and decoder calls, so its shapes are
  unaffected.
- **Nothing is inlined into a position that can run before its declaration**.
  One helper (`untwist_deobfuscate::init_order`) answers, for a declaration and a reference,
  whether the reference can only run after the declaration's initialiser: the reference must be a
  later member of the declaration's own statement list, or -- when it sits inside a hoisted
  `FunctionDeclaration` -- every statement before the declaration in that list must be unable to
  call, or that function's binding must be unreachable before the declaration (every reference to
  it, transitively through other hoisted declarations, later in the same list). Pattern B array
  inlining, array-alias resolution, `inline_constant_variable` and the decoder-alias half of
  `inline-decoder-wrappers` all ask it. The reference deobfuscator asks nothing: on
  `function g() { return arr[0]; } g(); var arr = ['x'];` it prints `'x'` where the program throws
  a TypeError. Effect: Untwist leaves a decodable site behind on a bundle that reads before it
  declares.
- **A simple string array is inlined only through reads used as values**. Every reference must be an element read with a literal index whose result is a
  value: a read that is called (directly, through parentheses, a sequence, a conditional or a
  logical operand), constructed, used as a tag, used as the base of a member access, iterated
  (`for…of`, spread, `yield*`) or destructured keeps the array, with a note, because the inlined
  literal would change the receiver, the error the call throws or the text of its message. A read
  used as a member key, an argument or an operand is a value.
- **What else may reach a string array is judged on the input's scoping**. The input, not the tree after an anonymous shell is stripped, decides
  how a binding can be reached: `game.js` is `(function anonymous(…) { … })`, so every binding in
  it is a local of that function even though it is top-level once the shell is gone.
  - A **nested binding** (any binding of a function, a block, a module or a stripped shell) is
    reached only by its own references, which the rule above judges, by a direct `eval` (a call
    whose callee is the identifier `eval`) in a scope that can see it, and by a `with` statement
    in such a scope; either keeps the array. Indirect code text, unknown-key calls and
    enumeration of the global object cannot see it.
  - A **top-level `var` or function of a classic script** is also a property of the global
    object. When the program may hold the global object (premise 2), the array is kept if any
    access, read or write, on a base that may be the global object has a key that may be its name,
    if an enumeration that yields values (`Object.values`/`entries`, descriptors, `for…in`,
    spread, rest destructuring, an `Object.assign` source, `JSON.stringify` with a replacer, an
    unknown-key call's arguments) is applied to such a base, or if code text may reach it. A base
    is not the global object when it is a fresh value, the result of a recognised string-array
    function, an element read of a watched array, or a parameter of a function that does not
    escape whose every call site passes such a value; an ambient name used as a base (`console`,
    `Object`, any standard global but `globalThis`) is its intrinsic unless a write anywhere may
    have that name as key, it is assigned, or the walk meets code text it cannot judge.
    Name-only observation (`in`, `hasOwnProperty`, `Object.hasOwn`, `propertyIsEnumerable`,
    `Object.keys`, `getOwnPropertyNames`, a descriptor read, `Reflect.ownKeys`, a printing or
    serialising call, or any call of a function the walk does not read, taking such a base as an
    argument) does not stop the inlining, but the declaration is then kept as written, because
    removing it would change the names the global object has.
  - A **top-level `let`, `const` or class of a script** is not a property of the global object,
    but every code text, indirect included, sees it; only code text, a direct `eval` and `with`
    are judged for it.
  - **Code text** is judged by folding each argument to a string (literals, concatenations,
    resolved decoder calls, element reads of sealed literal arrays and of watched arrays, plain
    object literals) and parsing it: text that spells a watched name, or whose own walk reaches,
    observes or runs code that reaches a watched name, keeps the array; text that does not fold
    keeps it; text that does not parse runs nothing. Forwarded runners (`.call`, `.apply`,
    `Reflect.apply`) are judged the same way.
  - An element read of a recognised string array yields a string or `undefined`, never a definer or
    a runner, so (A) does not count it as a value leak, even with a computed index.
  The decoder units follow the same reach rule for their string-array function. Pinned by
  `crates/untwist/tests/string_array_reach_rows.rs` (node rows, as CommonJS and as a script, both
  modes, including shell-wrapped programs compared with the shell called). Accepted losses: `fixtures/behaviour/second-unit-a-call-passes-a-non-literal-argument.js`,
  `second-unit-a-decoder-parameter-overwritten-with-plus.js` and `second-unit-b-decoder-called-in-a-print.js`
  leave their first unit undecoded: `console[key](arg)` with a key and an argument from another
  unit's decoder calls the decode cannot resolve is an unknown-key call whose argument may be the
  global object. Open: a decoder unit's machinery is still removed when the program observes the
  global object's names (it would change `h-*`, `high`, `j1` and corpus outputs through
  obfuscator.io's own `callControllerWrapper(this, …)`).
- **An array declaration is removed only when every read was substituted**.
  `find-string-array` counts the element edits it planned against the binding's read count and
  keeps the declaration, with a note, if any read was not rewritten -- so a failed element lookup
  can never leave `arr[i]` pointing at a declaration that is gone. Every pass in the stage also
  compares the edits it applied against the edits it planned and notes a mismatch.
- **An object literal that defines a key more than once is refused**. The reference deobfuscator's
  `inline-object-props` resolves a read to the FIRST matching property; JavaScript takes the last,
  so `var o = { k: 1, k: 2 }; o.k` is rewritten to `1` where the program says `2`. Untwist refuses
  the whole literal with a note, comparing keys after `prop_name` normalisation so `1` and `"1"`,
  and identifier and string spellings of one name, count as duplicates. A `__proto__` key is
  refused on the same path: it defines no own property, so no read of it is decidable from the
  literal. Effect: Untwist leaves such an object in place; the reference deobfuscator silently substitutes the
  wrong value.
- **The undefined value is printed `void 0`, not `undefined`**. `undefined` is a
  globally-bound identifier, not a keyword, so any scope may shadow it; Babel's `valueToNode` --
  and therefore the reference deobfuscator -- emits the bare identifier, which reads the shadowing value. Every site
  in the deobfuscate stage that materialises the undefined value goes through one untwist-core
  helper that builds `void 0`. The unminify stage's `void-to-undefined` turns it back into
  `undefined` for readability, but only in a scope with no `undefined` binding, no enclosing `with`
  statement and no direct `eval` in the program, so the pipeline's
  output matches the reference deobfuscator's everywhere the shadowing hazard is absent. Effect: a shadowing scope
  keeps `void 0` where the reference deobfuscator prints `undefined` and changes the value.
- **A non-string decode result is printed as itself, and an object refuses its call site**. `NaN`, `Infinity`, `-Infinity` and `-0` have no literal form, and Untwist
  previously substituted `undefined` for all four -- four silent value changes reported only by a
  note saying the site "did not decode to a string". They now build `0 / 0`, `1 / 0`, `-1 / 0` and
  `-0`, which is what Babel's `valueToNode` does. The one value with no expression form at all, an
  object, leaves its call site alone with its own note rather than becoming `undefined`.
- **The deobfuscate passes stay `Safety::Safe`, and that is now honest**. The reference deobfuscator tags `inline-decoder-wrappers` and
  `inline-decoded-strings` `unsafe` and runs them anyway; Untwist's `PassGroup::new` really does
  filter on the tag, so tagging them `Unsafe` would silently disable deobfuscation by default.
  Rather than relabel the passes or add a third tier, Untwist closes each hole those tags stand
  in for with a refusal and a note, as the bullets above describe.
  Where a heuristic is inexact, the pass declines and says so; it does not guess. A direct `eval`
  or a `with` statement anywhere in the program is reported once per run, because either can reach
  the decoder by a name this stage renames or removes; the reference deobfuscator has the same hole and says
  nothing.
- **The rotator is reported by the driver, not by a pass.** `find-array-rotator` used to be a
  whole-program traversal that produced nothing but a note; the note now fires from `run_round`,
  once per stamped array, alongside `NO_STRING_ARRAY_NOTE`. `find_rotator` is still computed
  independently where it is consumed, in `removal.rs` and `bridge.rs`.
- `evaluate-globals` substitutes a decoded `atob`/`unescape`/`decodeURI`/`decodeURIComponent`
  result whatever its code points, with **no** ASCII-only gate. The reference deobfuscator PR #157 proposed one; it
  was self-closed the same day and its author could not identify the bug it was meant to fix, the
  reported symptom was a Python re-parse of the output rather than a JS one, and an ASCII gate
  would silently skip every correct decode that yields Cyrillic, CJK or emoji text. `oxc_codegen`
  escapes the two characters that actually break a JS re-parse — U+2028 and U+2029 — and the
  lone-surrogate marker, verified against `oxc_codegen-0.148.0/src/str.rs`'s `ESCAPES` table; the
  round-trip is pinned by `crates/untwist-deobfuscate/tests/evaluate_globals.rs`.
- **`evaluate-globals` assumes pristine global functions; `fold-literal-chains` reads the
  decoder's write record.** `evaluate-globals`
  evaluates `decodeURIComponent('%41')` with the standard behaviour even when the program replaces
  it (`decodeURIComponent = function(){ … }`), as the reference deobfuscator does in both modes; node prints the
  replacement. `fold-literal-chains` (default mode only) folds a literal chain such as
  `'abc'.charAt(0)` or `[1, 2].join('')` only when no write in the program may change a key the
  chain consults. The check is target-agnostic: every member write,
  `delete` and definer call (`defineProperty`, `defineProperties`, `assign`, `Reflect.set`,
  `Reflect.defineProperty`, `Reflect.deleteProperty`, `setPrototypeOf`, `__defineGetter__`,
  `__defineSetter__` and the array mutators, whether called directly, through `.call`, `.apply`,
  `.bind` or `Reflect.apply`, or taken as a value) records the keys it may write, whatever its
  target, unless the target is a fresh object the check itself proves. A key it cannot resolve
  counts as any key. Code text that may write or run code (`Function`, `eval`, a function
  constructor reached through `.constructor`, a tagged template on any of these), a direct `eval`,
  a `with` statement, and a definer, reader or code runner taken as a value count as writing every
  key; reading the `__proto__` setter (`__lookupSetter__`, `getOwnPropertyDescriptor`) counts as a
  definer. A write is judged to miss a chain only when its target is a syntactic reference to an
  unaliased realm prototype (`Number.prototype.join = …` leaves a string fold), and that
  precision is dropped when a realm name may be rebound: an assignment to `String`, `Array`,
  `Object`, `Function`, `Number`, `RegExp` or `Symbol`, or a write on a base that is not a realm
  prototype under their name or an unknown key. Each of these judgements is fail-closed: it
  holds only for the exact shapes below, and every other shape bails. A helper is read at each call
  with its arguments in place of its parameters only when every reference to it is a plain call
  `h(arg, …)` with matching positional arguments; a reference as a member object (`h.call`,
  `h.apply`, `h.bind`, `h.prototype`), a tag, a spread call, a recursive call, or a parameter with a
  default or a pattern makes it escape, and the check reads its body as written; a top-level helper
  of a script is also read as written whenever the program may hold the global object (premise 2).
  `Symbol.<name>` is a well-known symbol only through the unshadowed global `Symbol` when the
  program neither assigns `Symbol` nor contains a write, on any base, whose key may be `Symbol`
  (premise 2); otherwise it is an unknown key. Under the same condition a binding whose only value
  is a call of that `Symbol` is a symbol key, which names no property of the global object. An
  object rest pattern reads unknown keys, unless its source is an object literal, or a binding to
  one that nothing writes or calls a method on and that is only read, spread or destructured: then
  it reads exactly that literal's keys, its spreads included. The same holds through a binding
  whose only value is another such binding, and for a literal built by keyed assignments
  (`const t = {}; t[k] = v, …; const o = t;`, javascript-obfuscator's transformObjectKeys): the
  assignments follow the declaration in the same statement list (function declarations between
  them aside), each key is a static or decoded string other than an `Object.prototype` name, and
  the assignments and the alias are the only references to `t`; `o` then reads the literal's keys
  and the assigned ones. A pending key counts only after it decodes.
  A direct `m.get(…)` or `m.set(…)` call is the
  native method, not `Reflect.get` or `Reflect.set`, when `m` is bound once to `new Map()`,
  `new WeakMap()`, `new Set()` or `new WeakSet()` with no arguments, through the unshadowed global.
  `m` must also be used only as `m.get`, `m.set`, `m.has`, `m.delete`, `m.add`, `m.clear` or
  `m.size`, never written, and never let out through the value of a `set` or `add` call. A use
  through a decoder call key (`m[d(1)](k)`) is judged by the string the call decodes to (the
  decoded-halves premise): one of those seven names counts as that use, any other string, or a
  call that does not decode, unseals `m`. The program must neither assign `Map`, `WeakMap`, `Set`
  or `WeakSet` nor contain a write, on any base, whose key may be one of those names, `get` or
  `set`; a write by a decoder call key counts by its decoded string, and one that does not decode
  may be any key (premise 2). On such a collection a `set`, `get`, `has`, `delete` or `add` call
  observes no names of its arguments, and `m.size`, `m.has(…)` and `m.delete(…)` hold no value,
  while no write may have that method's key. A binding bound once to `new C(…)`, where `C` is a
  class never assigned, with no `extends`, no decorators, a constructor that returns no value, no
  computed instance member and no instance field or accessor named `set`, `get`, `defineProperty`,
  `deleteProperty`, `getOwnPropertyDescriptor`, `setPrototypeOf`, `defineProperties`, `assign`,
  `has`, `delete`, `add` or `size`, and that is only read or called through `v.x`, never written or
  passed on, has its calls of those first eight names judged as the class's own method (a user function, judged
  where it is written) rather than as the `Reflect` or `Object` function of that name, while no
  write may have that name's key or `__proto__`. `Object.assign(t, …)` through the unshadowed
  `Object` returns `t`, so it is fresh when `t` is, while no write outside a fresh object may
  have the key `assign` or `Object`. A receiver reached through a field (`this.handlers.set`,
  `this.position.set`) is still judged by name: bounding it needs the flow of `this` and of fields.
  A webpack 4 or 5 bootstrap counts as proven when its module table is an array or object
  literal of plain functions that only its one require function dispatches, once, as
  `table[id](…)` or `table[id].call(…)` with every module id a literal key of the table, and the
  require function and its `d`, `m` and `c` properties stay inside the bootstrap (an AMD factory
  that declares no parameter for require may receive it). Then a module function's `module` and
  `exports` parameters hold objects, never the global object, when the require function creates
  the module record as an object literal with one plain `exports: {}` in the statement right before
  the dispatch, the dispatch runs first in its statement, every argument before them is a plain read,
  and the module never assigns the parameter. When the record is stored into the module cache on
  the way (`var m = (cache[id] = {…})`), the cache must not be reachable outside the require
  function except through its `c` store and `c` must never be read, since a setter on the cache
  could swap `exports` before the dispatch reads it. An accessor that `Object.prototype` inherits
  for a module id is not looked for: this is the premise of folding key assignments below. A webpack 5 `d(exports, definition)` that only copies `definition`'s keys
  as getters onto `exports`, guarded by `o(definition, key)`, reads only the functions of the
  object literals its calls pass, never a reflective value. `Object.create(…)` through the unshadowed `Object`
  also creates an object, as long as no write may have the key `create` and `Object` is not
  rebound. A proven dispatch is not read as a possible call of an unknown function on its
  arguments. A bootstrap that fails one of these shapes is judged as unknown code and noted.
  `Function` and `eval` reached through any forwarder
  (`.call`, `.apply`, `Reflect.apply`) with non-literal text count as code runners. An identity
  write (`O.k = O.k` on the same unaliased prototype, directly or through a substituted helper)
  writes nothing. A chain consults its method keys, the implicit keys (`toString`, `valueOf`,
  `@@toPrimitive`, `constructor` and the symbol methods its arguments look up) and, for a method
  that creates an array, `@@species`; an own key of the receiver's prototype is not reached through
  `Object.prototype`. Three premises bound the check. **1. Code from outside the file** (`require`
  of a relative path or of a built-in module such as `util` or `vm`, `import()`, other scripts on
  the page) is taken as writing nothing, so a host API that writes a realm prototype
  (`require('util').inherits`) is not seen.
  **2. Classic-script globals and the global object:** a top-level `var` or function is a property of
  the global object in a script, and code that holds the global object can read, call or overwrite
  it, or a built-in, at any time. The check does not follow where the global object flows. It asks one flow-insensitive question, whether any code may hold the
  global object, and the answer is yes when the program contains any of these sources: `this` or
  `super` at the top level of a script or in a non-strict function (its parameter defaults
  included), outside a class field or static block and outside a bound literal method (a method,
  getter or setter of an object literal that is the only value of a `const` or `let` binding, where
  every use of that binding, and of `this` in those bodies, is `o.k` with a static or decoded key,
  a method key is only ever called as `o.k(…)`, and the body neither reads `arguments` nor is a
  named function expression; a `const` or `let` literal built by keyed assignments and then
  aliased, as above, counts as the literal it builds through its alias, the functions it was
  assigned included); a reference to an undeclared name that is not a
  standard ECMAScript global (`Array`, `Object`, `String`, `Number`, `Boolean`, `Symbol`, `BigInt`,
  `Math`, `JSON`, `Reflect`, `Proxy`, `Promise`, `Date`, `RegExp`, `Map`, `Set`, `WeakMap`,
  `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Intl`, `Atomics`, `Iterator`, `DataView`, the
  errors, the array buffers and typed arrays, `parseInt`, `parseFloat`, `isNaN`, `isFinite`, the URI
  functions, `escape`, `unescape`, `undefined`, `NaN`, `Infinity`), `console`, or `arguments` inside
  a function, so `globalThis`, `window`, `self`, `global`, `Function`, `eval`, `require`, `module`
  and every host name are sources; `delete` of an undeclared name; a `with` statement; `import()`; a
  code runner called or forwarded, whatever its text, or read as a value (a key that may be
  `constructor` or that the check cannot resolve included); and a member or pattern key
  `prepareStackTrace`, or a write whose key may be `prepareStackTrace`. The last two kinds (a runner
  read, a key that may be `constructor` or `prepareStackTrace`) do not count inside a unit's own
  machinery: the base64 and RC4 decoders read their cache and their RC4 table by computed keys,
  and those reads and writes reach only objects the decoder created, so an encoded string array no
  longer makes a program a source that the same program with a plain array is not
  (`crates/untwist/tests/machinery_source_rows.rs`, node-checked; a source in the machinery's
  syntax itself, such as a sloppy `this`, still counts). A source refuses nothing by
  itself. When the program may hold the global object, the check assumes that unseen code reads,
  calls and writes every global property at any time, and judges what that can change by key, never
  by base. **Overwrite** (judged in every program): a top-level binding of a script is read as
  written when any write outside the decoder machinery, on any base, may have its name as key: a
  member assignment, compound assignment, update or `delete`, a destructuring or `for…of` target,
  `defineProperty`, `defineProperties`, `assign`, `Reflect.set`, `Reflect.defineProperty`,
  `setPrototypeOf` and the other definers above, where a key the check cannot resolve, including one
  a decoder call computes before the decode resolves it, may be any name. `Symbol` is unchecked (a
  `Symbol.<name>` key counts as unknown) when such a write may have the key `Symbol`, and the realm
  roots `String`, `Array`, `Object`, `Function`, `Number`, `RegExp` and `Symbol` count as rebound
  when such a write, on a base that is not a syntactic reference to a realm prototype, may have
  their name (a root is first rebound through such a base, or by assigning its name). **Hidden
  call:** every top-level function of a script is read as also called with unknown arguments: its
  body is read as written, besides its visible calls, which are still read with their arguments in
  place. A helper that writes nothing outside its own locals and fresh objects changes nothing; one
  that writes a parameter key on a parameter target counts as writing every key. **Hidden read:** a
  read alone changes nothing, but a top-level object or array literal of a script holds state, so
  the binding is read as written when any write outside the machinery may have one of its keys (an
  index or `length` for an array). `inline-object-props` and `merge-object-assignments` leave alone
  every top-level binding read as written and, when the program may hold the global object, every
  top-level binding of the script, because unseen code may read it at any time. The decoder's
  machinery is the evaluator's to model, so its own writes do not count as overwrites, but its
  sources count. A module's bindings are not global properties. **Filtered key enumeration**
  (javascript-obfuscator's domain lock): in `for (k in o) { if (p(k, …)) { v = k; break; } }` the
  keys `k` and `v` belong to a closed set instead of being unknown keys when `p` is a function that
  is never reassigned, is declared inside a function, and whose body (and the body of every function
  it calls) only reads its parameters and locals, compares, computes, and calls `charCodeAt`,
  `charAt` or a decoder call the decode resolves; `k` is passed to `p` once, beside literal
  arguments, and is otherwise only stored into `v`; `v` is declared without a value in the same
  function, written only there, and read only as a computed key, under `!` or `typeof`, or compared
  with a literal. The set is `undefined` and every string `p` does not return false for. For each
  name a check asks about (the machinery's names, the realm roots, `Symbol`, the definer, reader and
  runner names, `constructor`), `p` runs in a small evaluator with a step cap, and a bail counts as
  accepted. The enumeration is then neither a reach nor a name observation, and `o[v]` or
  `o[v][w] = x` reaches no name `p` rejects; removing the machinery cannot change which key the loop
  finds, because every removed name fails `p`. Any other shape keeps the key unknown. Rows:
  `crates/untwist/tests/filtered_enumeration_rows.rs`. **3. ECMAScript's intrinsics are a
  closed set:** `crates/untwist-deobfuscate/tests/intrinsic_walker.rs` walks every intrinsic
  function in node's realm (from the standard globals and the hidden intrinsics, through
  prototypes, statics and accessors) and fails on one that is not classified in
  `tests/data/intrinsics.tsv`, or whose keys the check does not judge by its class (definer,
  reader, mutator, runner, forwarder, construct). A call of a method named `get` or
  `getOwnPropertyDescriptor` judges its keys from the second argument on, where `Reflect.get`,
  `Object.getOwnPropertyDescriptor` and `Reflect.getOwnPropertyDescriptor` take their key, so
  `map.get(key)` is not a reflective read; a reader stored under such a name is already an escaped
  definer read. A keyed reader or writer called without its key argument, and with no spread at or
  before it, reads or writes the key `undefined`. Host APIs fall under premise 1. Writes that
  change only index keys (`Array.of` or a species constructor returning a prototype) are not
  judged: no fold reads a hole. A branch whose test folds through an object table is skipped:
  the table is an object literal, or one assembled as `var t = {…}; t[k] = v; …; var a = t;` by
  contiguous statements that are the only references to `t` (not a parameter), whose keys are
  literals or decoder calls resolved to the strings the decode predicts and are no own key of
  `Object.prototype`, and that no write outside the machinery on a base that may be
  `Object.prototype` may shadow; until a decoded value first differs from its prediction the test
  evaluates as folded, so the skipped branch never runs. The accepted precision losses are a binding
  that aliases another root under the same key (`var N = Number; N.prototype.join = …`), a
  prototype reached through a fresh root or the global object (`globalThis.Number.prototype.join =
  …`), and an unknown key on the `Array` constructor (it may be `Symbol.species`). `game.js` keeps
  17 foldable lines unfolded: its fifth write, Babel's
  `_defineProperty(obj, key, value)` (`key in obj ? Object.defineProperty(obj, key, …) : obj[key]
  = value`), writes a parameter key on a parameter target, so the check saturates there. Measuring what lies behind it, by removing each blocker from `game.js`'s default output in turn:
  900 more in the first 32,229 of its 203,933 lines, and the check still saturated. They are 451
  definer reads (414 escaped reads such as `Object.defineProperty` or `Reflect.get` taken as a
  value or a computed read whose key may be a definer, 37 reflective reads), 20 code-runner reads,
  260 calls read as code and 169 writes of an unknown key on a target the check does not bound,
  in core-js, Babel's helpers and the bundled libraries after them. Only 11 of them lie in
  core-js's `_export`, `defineBuiltIn` and `defineGlobalProperty`, so modelling the value flow
  through those three helpers would regain none of the 17 folds, so it is not built. The folds need a target-aware analysis, which this
  check gives up by design. Pinned by
  `crates/untwist/tests/realm_shapes.rs` (node rows, as a module and as a script, including 95
  shapes an earlier version got wrong), `crates/untwist/tests/fail_closed_rows.rs` (the forwarder,
  helper-escape, default, spread, recursion, `Symbol` and tagged-method shapes),
  `crates/untwist/tests/global_value_rows.rs`, `crates/untwist/tests/global_reach_rows.rs` and
  `crates/untwist/tests/key_judged_rows.rs` (a value that may be the global object reaching a
  top-level binding, a helper, an object's fields or `Symbol`, and the sources),
  `crates/untwist/tests/patched_builtin_folds.rs`,
  `crates/untwist-deobfuscate/tests/fold_chains.rs` and `decode_behaviour.rs`.
- **The use-before-initialisation gate's reachability analysis is syntactic.**
  `crates/untwist-deobfuscate/src/init_order.rs`'s `is_unreachable_before` decides whether a
  hoisted function can run before a declaration by following *named* references. An indirect call
  through a computed global — `(function () { globalThis["g"](); })(); var arr = ["EARLY"];
  function g() { return arr[0]; }` — is invisible to it, so that program is inlined although the
  original throws. No name-based static analysis can see the shape, the reference deobfuscator's included; a sound
  answer needs a call-graph over computed property access, which Untwist does not build.
- **An alias declared outside a machinery unit but read inside it keeps that unit's machinery**. `inline_variable_aliases` leaves machinery bodies untouched so that a kept
  machinery stays byte-identical to the input, and a depth-aware partial rename is unsound under
  the symbol-wide `Renames::apply`. No fixture has the shape and obfuscator.io cannot emit it into
  a rotator; the cost is one machinery unit that is kept rather than removed.
- **`control-flow-switch` refuses every fall-through, and three further shapes beside it**. The reference deobfuscator's flattener reorders such a case and silently changes the program; Untwist
  refuses instead. A `case` that does not end in `break`, `return`, `continue` or `throw` is
  refused whenever another case follows it in the switch, because the flattened sequence runs each
  case on its own turn and the body it falls into would be lost; the last case may run off the end
  of the switch only when its label is the last entry of the sequence. The three refusals beside
  it have the same character. The two prelude bindings — the `"1|3|2"` sequence string and the
  index that walks it — must each carry exactly one resolved reference, so a program that reads
  either anywhere else keeps its switch. A `for` used as the dispatch loop must be a bare `for
  (;;)`: an init or an update runs on every turn and flattening would drop it. And a case body
  that declares a `let`, `const`, `class` or function whose name is already bound in an enclosing
  scope is refused, because flattening lifts the case bodies out of the switch's own block scope
  and the declaration would collide there. Every refusal leaves the switch exactly as it was, with
  a note. This is the one entry in this list where Untwist deliberately does **less** than
  the reference deobfuscator, and it is a divergence towards correctness rather than away from it.
- **Folding an object's key assignments defines properties where the input assigned them.**
  `control-flow-object`'s transform-object-keys form rewrites `var o = {}; o["QuFtJ"] = …;` as a
  single object literal, and `merge-object-assignments` does the same for an ordinary object. An
  assignment runs `[[Set]]`, which walks the prototype chain and can call an inherited setter; a
  property written in an object literal is installed with `[[DefineOwnProperty]]`, which cannot.
  Reaching the difference needs an accessor inherited from `Object.prototype` whose name is
  exactly the five-letter alphabetic key the obfuscator generated, which no obfuscator emits, and
  no gate could rule it out in any case — that no prototype accessor exists anywhere in the
  program is not a question static analysis can answer. The reference deobfuscator makes the identical assumption
  here and in `inline-object-props`, so this matches the reference deobfuscator; the exposure is hostile hand-written input
  alone. The default lane makes the same assumption for a webpack module id when it stores a
  module record into the module cache (`crates/untwist/tests/data/webpack_bootstrap_premise_rows.txt`).
- **In the conservative comparison mode a string literal that a removed protection helper had demoted becomes a
  directive.** When the protections stage removes a helper from the head of a statement list,
  the reference deobfuscator prints the string-literal statements that now lead it as a directive prologue, and
  the conservative comparison mode reproduces that by promoting them. The input as given is sloppy code (the helper
  sits before the string, so the string is an ordinary expression statement), and the promotion
  makes the output strict: an assignment to an undeclared name throws, `this` in a plain call is
  `undefined`. Default mode keeps the string inert, printed as `("use strict");`, so the output
  stays sloppy. `fixtures/untranspile/hand/demoted-prologue-self-defending.js` (in a
  function body, game.js's shape) and `demoted-prologue-debug-protection.js` (at the program
  head) are the witnesses; their conservative lanes are allowlisted.
- **In the conservative comparison mode a simple string array's eligibility is decided once, before any decoding.**
  the reference deobfuscator's `findStringArray` runs a single time ahead of the decoders, so an array whose
  elements only become literals after decoding is never inlined. The conservative comparison mode reproduces that exactly:
  `var colours = ["#fff", "Hello World!"];` stays declared and its reads stay as `colours[0]`,
  `colours[1]`. Default mode decides eligibility again once the decoding has run and inlines the
  array, leaving `console["log"]("#fff", "Hello World!")`. Both are pinned in
  `crates/untwist-deobfuscate/tests/order.rs`.
- **`merge-object-assignments` refuses a value that can execute.** Folding `obj.k = v;` back into
  the object literal moves `v`'s evaluation up to the declaration, so a value carrying a call, a
  `new`, a tagged template, a dynamic `import`, an `await` or a `yield` anywhere inside it is
  refused: after the fold it would run while the binding is still uninitialised. A plain member
  read is accepted, so a getter on some *other* object does move with the fold — a documented
  residual, and one the reference deobfuscator shares, since it refuses nothing on this path at all.
- **A decoder-argument ternary the split cannot take apart reports nothing of its own.**
  `find-decoders` rewrites a decoder whose argument is a ternary into one declaration per branch;
  a branch it cannot split hands its symbols to the external-variable inlining that runs next,
  whose own refusals are noted, and a branch that stays unresolved surfaces downstream as a kept
  machinery unit with its own note. A note at the split itself would report the same shape a
  second time. The reference deobfuscator has no equivalent step and says nothing anywhere along this path.
- **A computed key that is no literal is merged in the conservative comparison mode and refused by default.**
  the reference deobfuscator's `merge-object-assignments` folds `obj[k] = v` into `{ [k]: v }` whatever `k` is,
  which is unsound twice over: `obj["__proto__"] = v` sets the prototype where `{ ["__proto__"]: v
  }` defines an own property, and a key expression that runs code now runs before the binding is
  initialised. The conservative comparison mode mirrors the reference deobfuscator and notes every adopted site, naming both hazards, with one exception: a key that is syntactically an object (an object or array literal, a function,
  class, `new` or regexp) is refused in both modes when the value is not inert, because the
  literal converts the key before evaluating the value while the assignment converts it after
  (`o[{ toString() { log += 'k' } }] = (log += 'v', 1)` logs `vk`, the merged literal `kv`).
  Refusing every key that might be an object moved the the conservative comparison mode game.js pin, so identifier,
  member and call keys are still adopted in the conservative comparison mode.
  Default mode merges a computed key only when it is an identifier whose binding is constant and
  initialised with a string, numeric or bare-template literal other than `"__proto__"`; every
  other key keeps its assignment statement, with the existing note. The conservative comparison mode is the
  byte-comparable oracle mode, so it adopts the reference deobfuscator's hazard on purpose; the default is a
  superset and never ships a known miscompile.
- **A dispatch-object forwarder that returns a logical expression is inlined by default only where
  every argument cannot throw.** `function (a, b) { return a || b; }` is recognised as a control-flow
  object property like its arithmetic siblings, but `a || b` evaluates `b` only when `a` is falsy, so
  moving an argument into the right operand can skip an evaluation the call performed. Default mode
  inlines such a call only when every argument can neither throw nor run code: a string, numeric,
  boolean or `null` literal; a bigint literal on its own, never under an operator; `void 0`, or an
  `undefined` that resolves to no binding; `!` over any of those; `-`, `+` or `~` over a numeric
  literal (nested, as in `- -1`); `===` or `!==` over any of those; any other binary operator only
  over a tree of such literals and unaries, with no identifier or bigint underneath, and never `in`
  or `instanceof`, which throw on primitives; or a bare identifier resolving to a `var`, parameter
  or function binding that a resolved reference proves is not an undeclared global, read at or after
  its own declaration — a parameter read inside an earlier parameter's default is a TDZ read that
  throws (`function f(a = o.aokfw(1, b), b)`), so a `var` or parameter reference that lies before its
  declaration is refused (a hoisted `var` read there could not throw, but the scoping
  tables do not tell the two apart), while a function declaration is initialised on scope entry and
  never is. Such an identifier may also sit under `!`, `===` or `!==`, which never run code, but not
  under an operator that converts it, because `ToNumeric` and `ToPrimitive` on an object-valued
  binding run `valueOf`, `toString` or `Symbol.toPrimitive`. A `let`, `const`, `class`, `catch` or
  import binding is refused (a shadowing `let undefined` included), and so is an unresolved
  identifier: skipping its evaluation would skip the `ReferenceError` the call throws. The conservative comparison mode
  keeps the reference deobfuscator's route, which substitutes any bare identifier and is unsound for exactly that
  case.
- **The default mode inlines a control-flow object whose writes all sit in a statically dead
  branch of its own dispatch.** The reference deobfuscator requires the binding to be constant. Untwist's
  default adds one relaxation: if the only writes to the binding are inside a branch the
  candidate's *own* dispatch proves unreachable, the object is still inlined and the dead
  writes go with it. The branch is decided only from a test that references the candidate —
  a call of one of its function properties with literal arguments, the receiver resolved by
  symbol so a same-named binding in another scope decides nothing, inlined and
  folded. A test with no reference to the candidate, a bare `"xx" === "yy"` among them, is
  never decided by this pass in any mode: removing such a branch is `dead-code`'s job, and
  `dead-code` is the crate's only `Safety::Unsafe` pass, so a user who withholds `--unsafe`
  gets no branch removal. A write inside a statement that is not itself wholly
  dead is left in place and noted, and a write or escape outside every dead branch refuses
  the candidate with a note of its own. The conservative comparison mode never takes this route.
- **The default mode recognises a dispatch object by its shape, not by its name.** The reference deobfuscator
  requires the obfuscator's own five-character `[a-zA-Z]{5}` key and binding names. Untwist's
  default also accepts a naturally named object with two or more properties where at least
  one is a forwarder function — `function (a, b) { return a + b; }` and its siblings — which
  is the shape the transform exists for regardless of what the obfuscator called it. Every
  other gate is unchanged, so a natural-name object is inlined only where a mangled one would
  be. The conservative comparison mode keeps the reference deobfuscator's name test. The same rule now decides a Transform-Object-Keys
  run: its assigned keys are gated once, as a list, rather than one at a time, so a naturally
  named `t.add = function (a, b) {…}; t.label = "total";` run folds exactly where the equivalent
  declared literal would.
- **A control-flow object property is inlined only at a call site the substitution can carry.**
  Independently of the LogicalExpression sub-case below, every call of a forwarder property
  must pass the general gate. The property is recognised at all only when
  every parameter is a plain identifier — no default, rest or destructuring pattern — and no
  name repeats. A call site may not pass more arguments than the function declares, and it is
  then substituted by one of two routes: the in-order route, which moves each argument into
  the body in parameter order, or the substitutable-argument route, which requires every
  argument to be a literal, a plain identifier, `void 0`, a simple unary (`-`, `+`, `!`, `~`)
  or binary expression over those, or a decoder call whose own arguments satisfy the same
  rule. A key-fold run that assigns a key the declared literal already defines is refused
  outright with the note `control_flow_duplicate_key: <name> is assigned to an object that
  already defines it`, because which value a read sees is not decidable from the folded literal. A
  call site that fails any of these keeps its call and the object is kept with it, reported as
  a partial fold. The reference deobfuscator substitutes by name with no such gate, which is how it miscompiles a
  spread argument or a duplicated key. Under `--unsafe` one spread is substituted as it does: a
  forwarder `function (a, b) { return a(b); }` called as `o.k(f, ...xs)` becomes `f(...xs)`.
  javascript-obfuscator emits that forwarder for `f(...xs)`, so the input passes `f` only the
  first element of `xs` and the output passes all of them, as the source did
  (`fixtures/obfuscated/webpack4-heavy.js`). Pinned by
  `crates/untwist/tests/unsafe_spread_forwarder.rs`.
- **What `cargo xtask diff-behaviour` actually compares.** Every fixture is deobfuscated in both
  modes — the conservative comparison mode with `--unsafe --no-unpack --no-jsx` and `--unsafe --no-unpack --no-jsx` — and input
  and output are run under node; a fixture is `behaviour ok` only when both modes agree with the
  input. The exit status and stdout must match exactly. stderr is compared after normalisation:
  each fixture path becomes `<file>`, every stack-frame line is dropped, node's location banner —
  the `file:line` header, the source excerpt and the caret — is dropped when it opens the text,
  and a trailing `Node.js v…` line is dropped. Text written with `console.error` is kept and
  compared verbatim, because the error's type and message are the behaviour while the path, the
  line and the excerpt are exactly what deobfuscation changes. An input that does not terminate
  within ten seconds, or that exits non-zero having written nothing to stdout, is skipped with its
  reason printed — decided from the input run alone, before the output is consulted, so the rule
  names no fixture and is not an allowlist. A program whose only observable behaviour is its own
  crash is no oracle for a transform whose job is to remove machinery.
- **`var-transformation` reports the refusals that are about meaning and stays silent on the ones
  that are about shape.** The pass turns an assignment to a smuggled parameter back into a `var`
  declaration, and it notes the three refusals a reader could not otherwise reconstruct: the
  parameter is read, or reachable from a nested function, before the assignment runs; it is
  written somewhere other than that assignment; or the function references `arguments`, which
  aliases its simple parameters. It says nothing when the shape simply is not the one the pass
  exists for — a right-hand side that reads the parameter itself (`e = e || {}`, `n = +n`) is
  normalisation and not smuggling; a function declaration inside the owning function that carries
  the parameter's name would collide with the rewritten `var` once hoisted; and any `.arguments`
  member access anywhere in the program can alias some function's parameters. A note for work the
  pass never claimed is noise — one Rollup bundle produces hundreds of them. The reference deobfuscator has no such
  pass and reports nothing either way.
- **`dead-assignment-elimination` collapses only a declaration and an assignment that sit in the
  binding's own statement list.** The empty `var` and its single write must both be members of the
  owner list — the owning function's body statements, or the program's top level — and never of a
  nested block, `try`, loop, `switch` or labelled list, because there a `throw`, `break` or
  `continue` can skip the write and still reach a read. At the top level the assignment must be
  the very next statement: a root-scope `var` is a property of the global object, reachable
  through `globalThis`, another script or an indirect read that carries no identifier reference
  this pass can see. A binding with any redeclaration is refused outright. The value admitted is a
  literal, or an expression Layer B folds to a constant, which includes pure builtin calls under
  the evaluator's pristine-builtins assumption — and a value admitted only through the fold must
  sit immediately after the declaration, since a literal has no evaluation to move but a folded
  call does. Class static blocks are never scanned, so a `var` declared inside one is refused
  silently. The reference deobfuscator has no such pass.
- **`fold-pure-calls` evaluates a call to a small pure helper when every argument is a literal.**
  The reference deobfuscator only decodes the javascript-obfuscator string-array shape, so
  `let dec = (a) => ["hello", "log"][a]; console[dec(1)](dec(0));` stays as it is there. This cleanup
  pass finds a function declaration, or a single-declarator `let`, `const` or `var` whose
  initialiser is an arrow or function expression, whose body is one returned expression, with
  simple parameters and neither `async` nor a generator. The expression is checked statically and
  then evaluated without running any input code: literals, parameters, unary, binary, logical,
  conditional, sequence and template expressions over primitives, and member reads of an array or
  object literal, a string, or a never-written binding whose initialiser is a flat literal. Any
  other node, a call, `this`, `arguments`, an assignment, a free name other than `undefined`,
  `NaN` or `Infinity`, or an operator applied to an object refuses the helper. A read only folds
  when it hits an own element or property: an index out of range, a hole, an inherited key such as
  `constructor` or a non-canonical index such as `"01"` leaves the call alone, so a call that
  would return `undefined` is skipped rather than folded, as are `NaN`, `Infinity` and strings
  over 64 KiB. Both lanes require the helper binding and every binding it reads to be written
  nowhere and declared once, an array or object it reads to be used only through member reads
  that are not a call, a tag or a `delete` target, the call to be a plain call with literal
  arguments, and the call to come after the `let`/`const`/`var` declarations it depends on in
  their own statement list, outside any function declaration nested in that scope (which may be
  hoisted and run earlier). A declaration in a `switch` case, a `var` in a nested block and a
  block-level function are never candidates. A helper whose every reference was folded is
  removed, and so is a binding it read that nothing else reads. A name that a block-level
  function also declares refuses the helper, as in the other inlining passes. For a top-level
  helper or binding of a classic script both lanes ask the global-reach question the simple string
  array asks: a write that may rebind one of the names through the global object or code text
  refuses the helper, and a read that may observe a top-level `var` or function through the global
  object keeps its declaration while the calls still fold. The safe lane further requires every
  reference to the helper to be a direct call. The unsafe lane drops that condition, assuming as the
  reference deobfuscator does that a function value handed elsewhere is not replaced, and its
  global-reach check, like every other one in that lane, takes a key the program cannot resolve to
  name nothing; `fixtures/pure-calls/kept-global-rebind.js` is a program where that assumption is
  wrong.
- **`fold-pure-calls` also runs a top-level helper that writes top-level state.** When no pure
  helper folds, the pass looks at the program's own statement list for a function declaration or
  single-declarator binding of an arrow or function expression whose body is a list of
  assignments and updates of top-level `let` or `var` bindings with a literal (or no)
  initialiser, plain expressions of the pure shape above, and one final `return`. The program's
  statements are then walked in evaluation order (a member callee before the arguments, left to
  right, into blocks, declarations and `if` tests); each call with literal arguments runs the
  body against the state's value at that point, and each read of the state becomes that value.
  The fold is all or nothing. It is refused when any reference to the helper or the state is not
  one the walk accounts for (another function, a closure, a getter, a value use, a write outside
  the helper, a call or read under `&&`, `||`, `??`, `?:`, optional chaining, a loop or `try`),
  when a call runs before a declaration it needs, when a body would throw or yield a value it
  cannot print, and, for a classic script, when the global-reach question above finds a write or
  an observation of one of the names. A call whose value is unused is removed with its
  statement. The helper and the state are removed afterwards, so the state's last value is never
  observable. Pinned by `crates/untwist/tests/straight_line_rows.rs` (node rows, both lanes) and
  `straight_line_sabotage.rs` (mutant `drop_straight_line_guards_for_tests`).
- **A binding that is declared twice is never constant.** `is_constant_binding` requires the
  symbol to have no redeclarations as well as no writes, because `var x = 1; var x = 2;` is two
  initialisations and a consumer that read the binding as constant would fold the first value into
  a site the second one reaches. The reference deobfuscator agrees — Babel counts a redeclaration among a binding's
  `constantViolations` — so this matches the reference deobfuscator, and it is recorded here only because Untwist briefly
  did not, and every pass that asks whether a binding is constant, `control-flow-object`'s
  declared form among them, inherited the wrong answer.
- **A binding that a block-level function's Annex B binding may reach is not inlined** (Annex B.3.3). Untwist parses the input as a module first, so its scoping
  binds a function declared in a block to that block. Sloppy code also gives the name a `var`
  binding in the enclosing function, or in the script at the top level, so reads there see
  `undefined` or the function, not the outer value. `find-string-array` (simple arrays and their
  aliases, by the alias's and the root's names) and `inline-object-props` refuse a binding, with a
  note, when a block function of its name sits in sloppy code and its enclosing function either
  is the binding's own function (the block function overwrites the binding, or conflicts with
  it) or lies inside the binding's scope and contains one of its reads. Code counts as strict, and
  such a binding is inlined, inside a function or arrow whose body starts with `'use strict'`,
  inside a class (its methods, accessors and static blocks), under a `'use strict'` directive at
  the top of the script and in a program with `import` or `export`. Anything else counts as
  sloppy, so a module without them is judged as a script, and async and generator block functions
  are judged like plain ones, though Annex B does not hoist them. The refusal is per binding, not
  per read: a binding whose reads all lie outside such a function, in a sibling function, or under
  a nested function's own binding of the name is inlined, but one read inside it keeps every read,
  those outside included. Annex B also skips the hoisting when a block between the function and
  the block function declares the name with `let` or `const`, as in
  `{ let arr = 1; { function arr() {} } }`. The check does not look for such a declaration, so it
  refuses there although node reads the outer binding.
  `control-flow-object`'s declared form keeps the name-wide rule, so a name any block function
  shares is refused there in strict code too, and the call judgement still treats such a name as
  opaque. Pinned by `crates/untwist/tests/annex_b_rows.rs` and
  `crates/untwist-deobfuscate/tests/annex_b_scope.rs` (node rows in both modes, and the rows that
  must still inline in both).
- **The call-substitution walk has a depth limit and a size budget**. Where it walks a
  helper's body again at a call, with the call's arguments substituted, it stops at 64 nested
  helpers or once the bodies it has walked add up to four times the input's size (at least
  64 KiB). Past either limit it reports the call as code it cannot follow, so string-array units
  and folds that depend on that walk are refused rather than guessed. Every fixture stays under
  half of that budget, and two helpers deep.
- **The conservative comparison mode is the oracle mode and the default is a superset of it.** Every pass Untwist has
  and the reference deobfuscator does not returns `true` from `requires_extended()`, and the conservative comparison mode filters
  those out, so the conservative comparison mode is what `cargo xtask diff-deobfuscate` compares byte-for-byte
  against the reference deobfuscator's snapshots and what the divergences above are measured in. A plain `untwist
  file.js` runs the superset as well — `var-transformation`, `dead-assignment-elimination`,
  `fold-literal-chains`, the relaxed dead-branch and natural-name dispatch-object arms, and the
  constant-key route through `merge-object-assignments` — so the default output is deliberately
  not byte-comparable with the reference deobfuscator's. The switch also decides how the stages are ordered. Under
  the conservative comparison mode, as in the reference deobfuscator, the cleanup passes run once after unminify and what they produce is
  never handed back, so a computed member a cleanup pass materialises — `x[atob("Zm9v")]` becoming
  `x["foo"]` — stays in that form, and conservative-mode output, like the reference deobfuscator's, is not guaranteed to be
  idempotent: running Untwist on it again can still change it. The default pipeline iterates
  unminify and cleanup to a fixpoint instead, prints `x.foo`, and settles. Wherever a sentence
  above says "by default" or "in the conservative comparison mode", that switch is what it names.
- **A pass drops its symbol-keyed state between fixpoint iterations.** `Scoping` is rebuilt after every
  changed iteration, so a `SymbolId` a pass cached in the previous one now names a different symbol —
  ids are dense indices, so a stale one is almost always valid and silently wrong. `Pass::reset` is
  called by `run_to_fixpoint` immediately after each rebuild and is where a pass clears exactly those
  fields; span-keyed state, such as `merge-object-assignments`' conservative-mode freeze, survives on purpose.
- **`inline-object-props` runs a second time in the cleanup group by default.** The reference deobfuscator runs it once,
  inside the deobfuscate composite, which is over before `merge-object-assignments` folds
  `var o = {}; o.a = 1;` into a literal and inlines it at its single reference. The `({ a: 1 }).a` that
  leaves behind is a read Untwist can resolve, so the default registers the pass again after
  `merge-object-assignments` in the cleanup fixpoint and the two settle against each other. The
  second registration runs on every default-mode input, including one in which no string array was
  found and the deobfuscate stage's own run of the pass was skipped; the pass's own gates — a readonly
  binding, a single reference, unambiguous keys — are what make that safe. The conservative comparison mode keeps the
  single registration; the reference deobfuscator produces the same `({ a: 1 }).a` shape, so this is a superset, not a
  correction.
- **Outside the conservative comparison mode, `inline-object-props` reads through a computed key that is not a literal.**
  the reference deobfuscator, and the conservative comparison mode, treat every `O[k]` with a non-literal key as an opaque use of `O`, so a
  constant map read that way is never inlined. `untwist_core::ast_util::readonly::scan_object_uses`
  now counts such a read in a plain expression position as a *keyed* read (`ObjectUses::keyed`)
  instead of only an opaque one, and the cleanup registration of the pass (the deobfuscate-stage
  registration and the conservative comparison mode are unchanged) classifies each keyed read of a literal-valued map
  (`crates/untwist-deobfuscate/src/object_props/keyed.rs`):
  - `delete O[k]` is a write, and so is a keyed read the scan could not find again; either keeps
    the map as it is.
  - A key that is a constant `var`/`let`/`const` binding, declared alone with a string or numeric
    literal, not at the program's top level (a script global can be rewritten through the global
    object), and initialised before the read on every path the init-order scan sees, resolves:
    `O[k]` is inlined like `O.lit`, in any position. This is exact, because the key is the literal
    and the map's own data property is what the read returns. It is the default.
  - Any other key in a callee, tag, `new` callee, call argument or `for`-`in`/`for`-`of` head
    position keeps the map: a callee or tag would pass `O` as `this`.
  - Any other key in a value position is an unknown-key read. Under `--unsafe` the map's static
    and resolved reads are inlined and its declaration is kept for the unknown-key reads. This is
    `--unsafe` only, under the assumption that **no accessor the program installs on
    `Object.prototype` runs when the map is read with a key it does not own** — such a getter
    receives the map as `this` and could write it (`fixtures/behaviour/object-inherited-getter.js`
    is the witness). A `toString` on the key runs before the read but reaches the map only
    through a reference the scan already counts.

  On `fixtures/real-world/game.js` under `--unsafe` this inlines 13 reads in the cleanup stage
  (three.js's `WEBGL_TYPE_SIZES.SCALAR`, five `PATH_PROPERTIES` reads and seven reads of a colour
  map), each map kept for its variable-key reads; the default output is unchanged. The
  `WEBGL_CONSTANTS` map (`GLTFLoader`) is kept whole, as before: the deobfuscate stage reads ten
  of its twelve keys behind decoder calls, and by cleanup all twelve reads are static, so it has
  no keyed read there and the pass's existing checks decide it as they did. The deobfuscate-stage
  registration stays at the reference deobfuscator's rule because a fire there costs a whole edit round (an
  application walk, a scoping refresh and the shared init-order scan) in a stage that otherwise
  makes no object edit on this input; the cleanup fixpoint pays those walks anyway.
- **A `function anonymous(…)` shell around the whole program is removed and its parameters are
  hoisted, whether it is the raw `toString` declaration form or the parenthesised expression
  form.** V8 prints `function anonymous(a,b\n) {\n…\n}` when a script is dumped through
  `new Function(...).toString()` — a `FunctionDeclaration` at program level — and a dump that was
  wrapped in parentheses on the way to disk parses as an uncalled function expression;
  `fixtures/real-world/game.js` is the parenthesised kind: one uncalled function expression holding
  the entire program, two levels of indentation deep, whose body reads both of its hashed parameters
  (`typeof` probes, two bare reads and one guarded call — the SDK hooks the caller passed in).
  Untwist removes the shell — in the pipeline, before scoping is built and before any stage runs,
  in both modes and with `--no-deobfuscate` — when the program is exactly that one statement
  (a declaration is never called by a program that holds nothing else), the function is named
  `anonymous`, it is never called, every parameter is a plain
  identifier (no default, no pattern, no rest), and none of these refuses it: a repeated parameter
  name; a parameter's name declared again by a `function` anywhere at its own level (a block-level
  function in a script would become a global the shell's scope never made, Annex B) or by a `class`,
  `let` or `const` at its top level (that would collide with the hoisted declaration; a top-level
  `var` of the same name is allowed); `this`, `arguments`, `new.target`, `super` or `import.meta` at
  its own level; a `return` at its own level (it would not even parse at program level); the
  identifier `await` at its own level (a script's `await(x)` would read as top-level await when the
  output is parsed module-first again); or any reference to its own name `anonymous`, at any depth
  (a named function expression binds its name inside its body, and the strip would leave the
  reference unbound). The body becomes the program and the parameters become a `var` declaration in
  front of it (`var`, because a parameter is function-scoped, hoisted and redeclarable), so for the
  no-argument call the shell describes every read yields `undefined`, every `typeof` yields
  `"undefined"` and every write binds where the parameter did. What that level *is* differs by
  source type: in a script the hoisted `var`s and the body's own top-level `var`s and functions
  become globals the shell's function scope never created; in a module (as `game.js` parses) they
  are module-scoped; the dump is never run, which is why the strip is acceptable in both. The guards
  are syntactic; the own-level scan stops at every nested function and the `anonymous` scan walks
  the whole body, which costs `game.js` about 18 ms. The strip is reported as a `strip-shell` stage
  row and a progress note naming the hoisted parameters; a shell the shape recognises but a later
  guard refuses is left in place with a refusal note, spanned at the function, saying which guard; a program that is not shell-shaped at all is silent. The reference deobfuscator keeps the shell, so
  `game.js` carries the one entry on `xtask/diff-deobfuscate-allowlist.txt`.
- **A rotation-memo hit reproduces one checksum evaluation's decoder state, not the search's**. Every deobfuscate round builds a fresh decoder session. The first round searches
  for the string array's rotation, which runs the checksum at rotations 0 to r; a later round
  whose machinery has the same fingerprint takes the memoised rotation and runs the checksum
  once, at r, to confirm it. Whatever the search *wrote* into decoder state (a property on the
  decoder function, a captured binding, a cache) is therefore not replayed on a hit. A decoder
  whose output depends on its own call history can decode the same call differently depending on
  the round that reached it. The program below (`b` counts its calls in `b.n`) prints
  `console.log("y6", "y5");` in both modes, where the un-memoised search (fe8e075) printed
  `"y6", "y6"`: the direct `b(0x3)` is decoded in round 1, which searched, and the same call
  through the wrappers `k` → `j` only in a later round, which hit. Neither is node's answer:
  node 22 prints `y2 y3`, because it never runs a rotation search at all. A stale hit leaks the
  same way: a confirmation that fails has already run one checksum before the fallback search
  starts. obfuscator.io's own decoder caches by
  `index + array[0]`, which is rotation-aware, so no corpus fixture moves. Gating the memo on
  "the search wrote nothing that existed before it" was measured and not shipped: the high
  preset's search writes 690 times to its decoder's cache (the `arguments` object priming left
  behind), so the gate would disable every hit there. Pinned by
  `a_memo_hit_reproduces_one_checksum_evaluations_decoder_state_not_the_searchs_so_a_call_history_decoder_decodes_differently_by_round`
  (`crates/untwist-deobfuscate/tests/rotation_memo.rs`).

  ```js
  (function (c, d) {
    var e = c();
    var k = b;
    while (!![]) {
      try {
        var f = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
        if (f === d) break;
        else e['push'](e['shift']());
      } catch (g) { e['push'](e['shift']()); }
    }
  }(a, 0xe));
  function a() { var l = ['x', '1', '2', '3', 'y']; a = function () { return l; }; return a(); }
  function b(c, d) {
    var e = a();
    return b = function (f, g) {
      f = f - 0x0; var h = e[f];
      b.n = (b.n || 0) + 1;
      if (f === 3) { h = h + b.n; }
      return h;
    }, b(c, d);
  }
  function hi() {
    function j(c, d) { return b(c - 0x1, d); }
    function k(c, d) { return j(c + 0x1, d); }
    console.log(b(0x3), k(0x3, 0x0));
  }
  hi();
  ```

- **A sink root passed as an argument to a program-declared helper, and only called from inside
  that helper, is outside the proof**. The escape detector
  exempts an argument position whose callee resolves statically to a function declared or
  expressed in the program (or, for a helper's own forwarding call, to a proxy object literal
  whose member is invoked with a resolvable key) from counting as an escape of `eval`, `Function`,
  `setTimeout`, `setInterval`, `setImmediate` or `execScript`. The exemption cannot see what the
  helper *does* with the parameter once inside it — whether it calls it plainly, stores it,
  forwards it again, or returns it to a caller the detector does not trace. A decode is trusted
  on the strength of this exemption alone; a helper that itself leaked the sink root somewhere
  unsafe is not caught. Real code relies on exactly this shape: the high preset's obfuscator.io
  polyfill passes `Function` into a proxy-object dispatch (`defineBuiltIn(Function, key, value)`)
  that the detector must not refuse, and refusing every helper call outright loses that decode
  (measured: `+13.12%` instructions, from the escape set falling back to every root). The premise
  is therefore accepted rather than closed: a one-level, syntactic helper trace, not a proof that
  the helper is safe.

- **A prototype derived from an unknown receiver's escape is outside the proof**. `x.constructor.prototype`, `Object.getPrototypeOf(x)`, `Reflect.getPrototypeOf(x)` and
  `x.__proto__` are tracked precisely for a *known* receiver — a literal, `new R()` for one of the
  six roots, a root identifier, or a never-written identifier whose declarator init is one of
  those (resolved once, post-walk, by locating the declarator via `Scoping::symbol_span` and a
  span-bounded search — never per node) — and such a value is fully covered by the counting rule
  and the accounted-or-Dynamic fallback, exactly like any other tracked alias. For an
  *unknown* receiver (anything else: a parameter, a call result, a member read whose object isn't
  itself known), every direct write through the derived value — a member write, a compound or
  update assignment, `delete`, a definer call, or a write credited through a tracked alias or a
  resolved helper summary — is still recorded exactly and refuses on a key conflict. Only the
  unaccounted-use/escape fallback is skipped for it: the value can be compared, stored, returned or
  passed on without being flagged Dynamic merely for not being visibly accounted for. Direct
  `R.prototype` for the six roots is never premised this way; only the four derived shapes above,
  and only through an unknown receiver, are. Real code relies on exactly this shape: core-js's
  `dom-token-list-prototype` module reads `documentCreateElement('span').classList`, then derives
  its prototype through `classList.constructor.prototype` inside a `&&` chain, returns it through a
  memoised getter, and its `dom-iterables` module later writes `forEach` and the iterator methods
  onto that returned value. Refusing this loses the decode of every core-js DOM-polyfilled bundle,
  including game.js's own main unit (measured: refusing this one shape costs 3 pins). The premise is
  therefore accepted rather than closed: a value whose receiver cannot be shown to be one of the six
  roots is trusted not to reach an unsafe write through a path the proof cannot see, exactly as the helper premise above
  trusts a helper's own use of its parameter.

- **A prototype alias is credited to its own binding, not to the first same-named one**. Two block-scoped aliases that share an identifier name but resolve
  to different bindings (`{ let P = Array.prototype; ... } { let P = String.prototype; ... }`) are
  each matched by the write's own reference `SymbolId`, not by scanning for the first entry whose
  *name* matches; this is a deliberate precision fix over the position-based lookup it replaced,
  which credited only the first same-named alias and fell back to `(root, Dynamic)` for every
  other one, refusing a unit whenever a later, differently-scoped alias's write went uncredited —
  strictly more refusals than the checks below now require, never fewer. See
  `crates/untwist-deobfuscate/tests/decoded_prototype_aliases.rs`'s
  `a_same_named_shadowed_alias_*` and `a_decoder_reading_the_first_of_two_same_named_shadowed_aliases_key_refuses_the_unit`
  rows, node-verified.

- **A rotator that runs before its machinery is bound is left alone**. `fixtures/reference/deobfuscate/obfuscator.io-var-functions.js` binds the string
  array as `var c = function () { … }` after the rotator `(function (b, e) { … f = b(); … })(c, …)`,
  so when node reaches the rotator `c` is still `undefined` and the program stops with
  `TypeError: b is not a function`. The reference deobfuscator decodes it as if the rotation had run; decoding it
  would turn a program that throws into one that prints. Untwist requires the
  string array and every decoder of a rotated unit to be function declarations, which are
  initialised before any statement runs, and refuses the unit otherwise
  (`MACHINERY_NOT_INITIALISED`, behaviour fixtures `var-decoder-declared-after-rotator.js` and
  `var-array-assigned-after-rotator.js`). The fixture is
  allowlisted in `xtask/diff-deobfuscate-allowlist.txt`; Untwist's output throws the same
  `TypeError` as the input.

- **Another unit's proven machinery before a rotator is inert**.
  Two concatenated obfuscator.io outputs put a second string-array unit's rotator after the first
  unit's rotator or a print of its decoded strings, which would otherwise refuse the second unit
  (`CODE_BEFORE_ROTATOR`). Before unit B's rotator Untwist also admits unit A's rotator, and
  an expression statement whose calls are A's decoders with literal arguments, bare or as the key
  and arguments of one method call on the unresolved global `console`, when A's decoders read only
  A's array, every rotator of A is sealed, A's array and decoders are function declarations,
  and A's machinery passes the base allowlist: no write to a base it did not create, no
  sloppy `this`, no global-reaching name, and no computed key anywhere in A's machinery that is
  not proven numeric. A decoder parameter counts as numeric only when the first
  statement of its function assigns it the result of `-`, `*`, `/`, `%`, `|`, `&`, `^`, `<<`, `>>`,
  `>>>` or unary `+`/`-` with a numeric literal operand, and every other write to it is of the same
  kind; `+` never qualifies. B is also decoded only when A decodes in the same run: if A is refused,
  bails in the evaluator or leaves a call undecoded, B is refused with `CODE_BEFORE_ROTATOR`. A
  unit whose own decoder is called in that print, or whose decoder is passed a non-literal argument,
  is refused (behaviour fixtures `second-unit-*`).


- **A string array is decoded only where its state at every call site is proven.** The reference deobfuscator's `findArrayRotator` needs one `try` per loop and a `parseInt` call;
  any other rotator is missed, and the reference deobfuscator then evaluates every call against the unrotated array,
  so the output prints something else (all 24 rotated cases of `fixtures/eval`, when their calls
  are printed). Untwist decodes a unit only when: its array function and decoders are named in no
  string, template or static key (else they are not even renamed); its array is referenced only by
  its declaration, its decoders and its rotator; its decoders only read the array and otherwise
  only their own bindings and standard globals; its rotator, if any, is a statement of the program
  body preceded only by declarations whose initialisers are on an allowlist of expressions that run
  no code, reads and writes nothing but its own bindings, the array and the decoders, and does not
  throw; the program replaces none of the built-ins the decoders and rotator use and writes nothing
  to a realm prototype that the decode consults; when the decode looks a key up through a realm
  prototype, no write the reading records may set a prototype's own prototype (a `__proto__` key,
  a computed key, `setPrototypeOf`), and the program runs no code text that the reading cannot
  resolve or that writes a property; and no decoded string names the machinery (else
  the original names are restored). It also recognises a loop whose `try` pushes and shifts the
  array in both blocks and breaks out of the loop, and brute-forces the rotation only for the exact
  obfuscator.io template, interpreting any other rotator. Every refusal is a note naming its
  reason. **The premise**: the same target-agnostic check as `fold-literal-chains` (above) judges
  every write outside the machinery against the keys the decode looked up on a realm prototype. A write whose key may be one of them refuses the unit; a `__proto__` key, a key the
  check cannot resolve, `setPrototypeOf` and code text refuse every unit that read a prototype; and
  a unit whose decoders use `instanceof Object` or a built-in prototype's own prototype is decoded
  only when no write may name `__proto__`, `@@hasInstance` or `Object`. The machinery
  itself is the evaluator's to model, so its own writes do not count. The same three premises
  apply, with the decoder's own premise below. A built-in the decode reads by name (`String`,
  `decodeURIComponent`) is still judged by key as well as by root: the unit is refused when any
  write in the program, on any base, names that built-in or a key the unit's machinery reads on it.
  Some keys and code texts are built only from string literals, `+`, template literals and calls:
  `String[d(1)]`, `Function(d(1) + d(2) + ")")`, `globalThis[d(1) + "x"]`. Such a key or text is
  judged after the decode, as its spelled twin, with the strings the unit itself decoded. This
  holds when every call in it is one of the unit's own decoder calls. A key that decodes to
  `constructor` or a code name still counts as code. A control-flow storage object counts too: an
  object literal held by a binding that is only read through static or decoder call keys, never
  written, deleted or passed on, outside any `with` or direct `eval`. Its member read is that
  member's text, and a call of its `function (a, b) { return a + b; }` member is the concatenation
  of the two arguments; a decoder call key is resolved first, and a key naming any other member
  leaves the text unknown. A call of such an object's `function (a, b) { return a(b); }` member
  with `Function` or `eval` first is judged as that runner called with the rest. Any other part,
  such as a parameter or a method call, leaves the key unknown. Code inside a class body is strict, so
  its `this` is never taken for the global object.
  Pinned by `crates/untwist/tests/decode_behaviour.rs` (node-checked, both modes, as a module and
  as a script), `crates/untwist/tests/alias_resolver.rs`, `crates/untwist/tests/realm_shapes.rs`,
  `crates/untwist-deobfuscate/tests/instanceof_object.rs` and the files of `fixtures/behaviour/` in
  `cargo xtask diff-behaviour`.

- **Under `--unsafe` the built-ins are assumed pristine, even when the program visibly patches
  one, as the reference deobfuscator assumes.** The rule above refuses a unit when a write in the
  program, code text it cannot resolve, or a runner or definer it cannot follow may replace a
  built-in the decoders or rotator use. In the unsafe lane that check does not run: the unit is
  decoded with the standard built-ins, whatever the program writes. A program that really
  patches such a built-in, such as `String.prototype.charCodeAt = …` after the rotator, prints
  something else after `--unsafe`, as it does after the reference deobfuscator; the default lane
  and the conservative comparison mode still refuse it and print what node prints.
  javascript-obfuscator output whose source takes `require` (`fixtures/obfuscated/`, and the
  reduced `fixtures/unsafe-parity/`) is the common case: holding a code loader may patch anything,
  so the default lane keeps its string array. Pinned by `crates/untwist/tests/unsafe_parity.rs`,
  and by the patched rows of `fail_closed_rows.rs`, `key_judged_rows.rs` and `realm_shapes.rs`,
  which print node's output by default and the pristine decode under `--unsafe`.

- **Replacing a decoder through a name built from another unit's decoded halves is the stated
  premise and is not refused**. A program
  assembles a decoder's name from two strings another unit decodes and runs
  `Function(name + ' = …')()`; Untwist decodes the string array as it was written, and node prints
  what the replacement returns. A program that does this on purpose is outside what any static
  deobfuscator can decode. Two shapes the premise would cover are refused, because a top-level string-array function of a script is a property of the
  global object: enumerating the global object in a classic script,
  `var g = Function('return this')(); for (var k in g) g[k] = …`, is a `for…in` over a
  base that may be the global object, and the own-unit form, `var s = qd(0) + qd(1);
  Function(s + ' = …')()`, runs code text that does not fold. Both now print what node prints.
  `decode_behaviour.rs` pins all three in both modes:
  `a_decoder_name_built_from_two_decoded_halves_is_the_documented_premise`,
  `a_decoder_replaced_through_global_enumeration_is_refused_and_prints_like_node` and
  `a_decoder_name_built_in_its_own_unit_is_refused_and_prints_like_node`.

- **Decoder machinery is kept while code may observe the names of the global object**. In a classic script, a top-level string array function,
  decoder or rotator is a property of the global object, so removing it changes what
  `Object.keys(this).length`, `Object.getOwnPropertyNames(globalThis)`, `'a' in this` or a
  `for…in` over the global object print. The decoded strings are still inlined, and the machinery
  stays stamped while observation lasts. Observation is re-judged at the end of every deobfuscate
  round (once per program change), again after unminify, and a last time in a final sweep after
  cleanup (and after the settle stage outside the conservative comparison mode); the machinery is removed at the first of these
  where no site may observe global names. `--rename` leaves the stamped names alone, and machinery
  that survives the final sweep is unstamped and then renamed by the same renamer, so renamed output
  stays idempotent. A stamp never takes a name the input already spells, as a binding or as a free
  reference, so `typeof __STRING_ARRAY__` in the input keeps node's value, and a
  `--report` note that the machinery was kept is dropped when a later sweep removes it. A site observes global names when it is an enumeration or name test of a base
  that may be the global object, or an unknown or printing call taking such a base as an argument.
  A base is proven not to be the global object when it is a primitive, a template literal, a fresh
  object, array, function or class, a decoded string, a `.length` read on an array or string literal or on a never-written, non-escaping local holding
  one, or the result of a visible function whose every `return` is proven the same way (through
  its call sites' arguments for parameters, to a bounded depth). An undeclared callee, or an
  argument rooted at an undeclared name, that no write and no unsafe code text may create throws a
  ReferenceError before the call runs; `Object.prototype` members other than `hasOwnProperty(k)` and
  `propertyIsEnumerable(k)` (which observe only `k`) do not enumerate; and an argument passed to a
  visible, non-escaping function that only uses it as the receiver of `fn.apply(p, arguments)` or
  `fn.call(p, …)` is not observed. A call to an intrinsic that neither enumerates nor prints its
  arguments (`JSON.parse`, `Math.max`, `parseInt`, `Number`, `String.fromCharCode`,
  `Promise.all`, `Promise.resolve`, the URI functions and the rest of the closed list
  `QUIET_INTRINSICS` in `calls/agnostic/quiet.rs`, each one a `pure` entry of node's realm walk) is
  not observed while no write may rebind the built-in or its key (a `JSON.parse = …` or a write by
  an unknown key makes it count again; `Promise.all` also waits on `then` and `resolve`). **The
  outside-code premise (PN-outside)**: code from outside the file does not depend on the names of
  the program's machinery, since the obfuscator picked them after that code was written. A call to
  `require` itself, or to a member of a never-written binding of `require("path")` or
  `require("url")` (with or without `node:`), modules that neither print nor enumerate their
  arguments, is therefore not observed, again only while no write may rebind `require` or that
  member; a removal that relied on it carries the progress note `the machinery was dropped under
  the outside-code premise`. The premise does not cover outside code that prints the global object
  (`require("util").inspect(globalThis)` observes, since `util` is not on the list) nor the
  program's own code. Pinned by `crates/untwist/tests/observer_rows.rs` (node-checked; every `ref-`
  row diverges when every callee is taken as quiet). Anything not proven counts as observation, and the machinery is
  kept with the note `remove-decoder-machinery: code may observe the names of the global object …`.
  The rule is the same in both modes. **Node-safe precision loss**: the self-defending code that
  survives the default mode in `fixtures/reference/deobfuscate/obfuscator.io-high.js` disables the
  console by passing console methods read off the global object to an unknown `.bind`, which no
  proof clears, so the default mode keeps its machinery (25,856 bytes instead of 11,688);
  `--unsafe` removes that code first and then the machinery, as before. A call controller behind
  an unfolded control-flow `if` (`if ("frofo" !== "UwkrO")`) is recognised in the default mode: its `return`s, its parameter uses and the function it returns are read through
  the branch the string-literal comparison takes, so `fixtures/corpus-npm/case-007.js` is
  3,491 bytes. `fixtures/behaviour/51-harmless-flatmap-polyfill.js`
  keeps it in both modes (813 bytes instead of 265), because `console.log([1, [2]].flatMap(f).length)`
  prints a `.length` read on a call result that the overwritten `flatMap` returns. Pinned
  by `crates/untwist/tests/machinery_name_rows.rs` (node-checked, both modes, as a module and as a
  script).

- **Name observation.** Four shapes would let a printing call see the global object without
  the machinery, so node would print `a` and `b` where Untwist's output did not:
  - *A method on a base that may be the global object.* `x.K(…)` with `K` no property of the
    global object throws only when `x` is the global object; otherwise `K` may be a printer
    (`var o = console; o.log(this)`, `[console][0].dir(this)`, `arguments.callee(this)`). Its
    arguments are now judged like those of any unknown call.
  - *A value that holds the global object.* `console.log` and `util.inspect` print nested values,
    so `console.log({ g: this })`, `console.log([this])`, an object whose field is written, a
    constructed object, a returned object, a function whose property is written through an alias
    or by a helper, and an object with a `util.inspect.custom` method all print its names. An
    argument of an unknown call now counts as observation unless it holds no reference to the
    global object: a primitive, a decoded string, an inline function or class without static
    members, or a literal whose values hold none (functions inside a literal count as holding one,
    since a getter or a custom inspector may return it). Because a prototype can give every
    object, array, function or regular expression a custom inspector or an iterator
    (`Array.prototype[Symbol.iterator] = function* () { yield globalThis; }`), an inline or bound
    object, array, function, class or regular expression counts as holding none only while no
    write in the program stores a value that may hold the global object, no write with a computed
    key stores a non-primitive, and no write goes through `.prototype` or `__proto__`. A class
    with an accessor, a static field or block, or a computed member always counts as holding one
    (`console.dir(cls, { getters: true })` runs a static getter). A parameter that is reassigned,
    or that `arguments` may alias, is never replaced by its call site's argument.
  - *A name created at run time, or defined by the host.* `this.zz = console; zz.log(this)`,
    `globalThis['zz'] = …` and `new Function('zz = console')()` create the undeclared name, so
    the ReferenceError rule now waits for the key rule and for any unsafe code text. A name the
    host defines (`console.log(this, URL)`, `new URLSearchParams(this)`) never throws: the rule
    applies only to names outside every global of node v22 (its global object and prototype chain,
    the CommonJS wrapper names, the built-in modules node exposes) and a list of browser globals
    (`calls/agnostic/hosts.rs`). A name some other host defines is the residual risk.
  - *Script text judged as a function body.* `vm.runInThisContext`, `runInContext`,
    `runInNewContext`, `vm.Script` and the timers run their text as a script, whose `var`
    declarations bind global names. Only `Function`, `x.constructor`, `vm.compileFunction` and
    `Module._compile` texts are judged as function bodies now.

  Two proofs keep the call controllers clear under the corrected rule: `f.apply(…)` and
  `f.call(…)` on a function value with no written field, or on a parameter that every call site
  binds to one (an `h = null` reset allowed), call user code and observe nothing unless a write may
  name `apply` or `call`; and a parameter of the function an immediately invoked function returns
  (one `return`, of a function expression) resolves through the call sites of the binding that
  holds it. An object-literal forwarder `{ K: function (f, a) { return f(a); } }` is judged at
  each call site that resolves to it (`o.K(…)` on a never-written, non-escaping object) as the
  call it forwards, and not in its body: any other way to reach it is an unknown call
  whose arguments are judged anyway. So `o.K(controller, this, fn)` is cleared exactly when
  `controller(this, fn)` is, and `o.K(console.log, this)` still observes.
  **Node-safe precision loss**: a core-js-style polyfill method `function (n) { return
  this.charAt(n); }` now keeps the machinery in both modes (`deobfuscate_retention.rs`), because
  `this` may be any receiver its caller chooses. No pin moved. Pinned by
  `crates/untwist/tests/machinery_observation_rows.rs` (node-checked, both modes, as a module and
  as a script, with a kept-machinery row per route).

- **An undeclared name that only some other host defines is assumed to throw**. The ReferenceError rule of the name-observation proofs knows every global
  of node v22 and a list of browser globals (`calls/agnostic/hosts.rs`). A name outside that list
  that another host defines, such as a Deno or Bun API, a worker-only global or an extension API,
  is taken to throw before the call runs, so a call such as `hostOnlyLogger(this)` does not count
  as observation and the decoder machinery may be removed. On that host the output can then print
  different global names than the input. Under node and in browsers the rule is exact.
