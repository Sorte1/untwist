# Known divergences from the reference deobfuscator: Unminify

- `block-statements` leaves `IfStatement` alternates (the `else` branch) unwrapped when they are a bare `if`; wrapping them fought `merge-else-if` and prevented fixpoint convergence. Output: `else if` chains stay flat, as in the reference deobfuscator's final output.
- A brace-less `while (x) var a = (b(), c())` whose sequence gets hoisted keeps the hoisted statements inside a block instead of a bare body (a bare body cannot hold two statements).
- `remove-double-not` also strips `!!` in loop tests (`while (!!x)` → `while (x)`), which the reference deobfuscator leaves alone; the boolean context makes it semantics-preserving.
- `default-parameters` only rewrites `arguments[i]` for `i <= 256` (`MAX_PARAMETER_INDEX`); higher indices are left as written.
- In the conservative comparison mode only, `default-parameters` turns a TypeScript-style `if (b === void 0) { b = a + 1; }` guard, or a Babel `arguments` read, into a default parameter as the reference deobfuscator does, even when that changes the function's `length`: a default parameter ends the count, so `function (a, b) { if (b === void 0) … }` has `length` 2 and `function (a, b = a + 1)` has 1, and an argument read at a new index grows it. Without the conservative comparison mode (default and `--unsafe`) the pass refuses any rewrite whose result would have a different `length`. `fixtures/untranspile/ts/lexical.es5{,-iter}{,.min}.js`, `fixtures/untranspile/hand/default-parameters-length.js` and the TypeScript and loose Babel files under `fixtures/untranspile/params/` print it and are on `xtask/diff-behaviour-allowlist.txt` for the parity lane.
- **`default-parameters` refuses a guard whose default would run somewhere else or see other bindings as a parameter default.** In every mode: an arrow function, a body with a directive (`"use strict"` cannot follow a non-simple parameter list), `await` or `yield` in the default, a name another parameter already has, an occupied slot that is not the same binding, a default that reads a binding of the body's own scope or a later parameter, a duplicate parameter name, a body-level redeclaration of the name, a function declaration in a nested block of a sloppy function, an accessor, and a program with a direct `eval` or a `with` statement. By default (and in the conservative comparison mode) also a generator unless the default is a literal, and a sloppy function that still uses `arguments` other than `arguments.length` or an index past the parameters: a default parameter unlinks `arguments` from the parameters. `--unsafe` assumes **nothing reads a parameter back through `arguments`**. The reference deobfuscator rewrites all of these; several then throw a `SyntaxError` or read a different variable.
- `template-literals` (string concatenation → template literal) is classified `Safety::Unsafe` and off by default; enable it with `--unsafe`. This is the reason `fixtures/unminify/strings-templates.js` is on `xtask/diff-allowlist.txt`.
- Computed-key matching (`keys_match` in `crates/untwist-unminify/src/transpile_util.rs`) never treats a static key (`a.x`) as equal to a computed key (`a["x"]`), even when the literal value matches: `MemberKey::Static` and `MemberKey::Computed` are only ever compared within their own variant (`Static` vs `Static` by name, `Computed` vs `Computed` by structural content equality, so `a[x]` matches another `a[x]` with the same identifier but never matches `a.x` or `a["x"]`). The reference deobfuscator's `isEqual` is looser and treats `a.x` and `a["x"]` as the same key. Effect: `logical-assignments` and `nullish-coalescing-assignment` skip merging an assignment pair when one side uses dot access and the other uses a matching string-literal computed access, where the reference deobfuscator would merge them.
- **`nullish-coalescing`, `optional-chaining`, `logical-assignments` and `nullish-coalescing-assignment` merge a repeated operand only when reading it once is the same as reading it twice.** By default (and in the conservative comparison mode) the operand must be `this`, a literal or an identifier that resolves to a declared binding; a member target (`a.b ||= c`, `a[k] ??= c`) needs such an object and a static, literal or such-identifier key, and each identifier in it must hold still while the first read runs: the input evaluates `a` and `k` again after reading `a.b`, and a getter there can rebind them. So the binding is never written after its declaration, is not imported, is not a `var`, `let`, `class` or function at the top of a script (the global object, another script or an indirect `eval` can replace those), and is not a `var` or parameter of a function that reads `arguments`. `o.p != null ? o.p : d`, `o.p && o.p.q`, `window.x || (window.x = v)` and an operand that is an undeclared global stay as written: a getter, a proxy or a global-object accessor may run code, and it runs twice in the input and once in `o.p ?? d`. `--unsafe` also merges member chains free of calls, updates and assignments, under the assumption that **reading a property has no effect**. The temporary-variable forms transpilers emit (`(_a = o.p) !== null && _a !== void 0 ? _a : d`) read once and convert in every mode. The reference deobfuscator merges any structurally equal pair.
- `!!0` folds only to `!true` (never to `false`) to match the reference deobfuscator's output byte-for-byte; fixpoint iteration then folds the remainder where the reference deobfuscator's other passes do.
- `number-expressions` (constant folding of unary negation and binary arithmetic on numeric literals) never materialises `-0`, `NaN`, or `Infinity` as literals; those stay as the original expression, because `untwist_eval::fold::const_to_expression` returns `None` for non-finite values and `number-expressions` refuses to emit negative-zero literals.
- The printer (oxc_codegen) keeps parentheses around IIFEs (`(function(){})()`), which the reference deobfuscator's printer drops; semantically identical. `fixtures/reference/unpack/browserify-cocos2d.js` carries this divergence on `xtask/diff-allowlist.txt` (the `--no-unpack` differential), where the container IIFE with the paren difference is still present in the output. It is not on `xtask/diff-unpack-allowlist.txt`: once unpacked, that container IIFE is dissolved into `bundle.json` plus module files, and `diff_unpack` never inspects `deobfuscated.js`, so the fixture compares identical there.
- Lone-surrogate string literals (`"\ud800"`) are never merged, folded, or converted to template literals; the reference deobfuscator corrupts them. Untwist preserves them verbatim.
- `number-expressions` folds `**` only when the result is provably exact (an integer power at most 2^53, or one of the spec's NaN, zero and infinity cases), in default and in the conservative comparison mode. The reference deobfuscator folds every `**` with node's own operator, so `10 ** -5` stays as written here and becomes `0.000009999999999999999` there; both run the same.
- `sequence` does not move a comma update into the body of an empty-bodied `for` whose head declares with `let` or `const` when the update contains a function, arrow or class: moved there, it would run before the per-iteration copy and its closure would capture a different binding. The reference deobfuscator's `sequence.ts` moves it and changes what the program prints; Untwist keeps it in both modes. The update also stays when the closure is in the loop's test, and the leading parts of a `for-in`/`for-of` head or a `for (let …)` initialiser stay in place when they hold a function, arrow or class, since hoisting them would rebind the closure from the loop binding to an outer one. They also stay when they read any name the lexical head declares: such a read runs in the binding's dead zone and throws, and hoisted it would read the outer binding instead (`for (let x of (x, [1]))` throws in node and printed "no" after the hoist).
- `sequence` does not move the leading parts of a sequence out of an assignment's right-hand side when they can touch the target (in both modes). The reference deobfuscator's `sequence.ts` turns `T op= (a, b)` into `a; T op= b`, which runs `a` before the target is read: `x += (x = 5, 1)` gave 6 instead of 2, `o.k += (f(), 1)` with `f` writing `o.k` read the new value, and `o.k = (swap(), 5)` with `swap` reassigning `o` wrote to the new object. Untwist still moves `x = (a, b)` with an identifier target. Otherwise the moved part must not write any guarded name (a compound identifier target, a member base, a computed key identifier) by name, and if it can run code at all (a call, `new`, a tag, a getter or setter, a coercion, `yield`, `await`) each guarded name must be a local of the current function that no other function writes, in a function with no direct `eval` and outside any `with` in it. A compound member target also refuses any member write or code in the moved part, since that can reach the property through an alias. So `a.x = (b(), c())` keeps its sequence when `a` is global and hoists when `a` is an uncaptured local, which keeps every game.js pin.
- `sequence` and `merge-object-assignments` can change a function's inferred `name`: `var f = (0, function () {})` is split into `0; var f = function () {}`, which names the function `f` where node names it `""`, and `o.p = function () {}` merged into `{ p: function () {} }` names it `p` where a member assignment names nothing. Both modes; the reference deobfuscator does the same. Refusing either shape moves the game.js pins in every mode, so the shapes are recorded rather than refused; probes `3-6cr-g18`, `g20` and `h22` show them.
- `json-parse` leaves `JSON.parse(text)` alone when `text` has an object key `__proto__` at any depth, in default and in the conservative comparison mode: the literal would set the prototype where `JSON.parse` makes an own property. The reference deobfuscator rewrites it and changes what the program prints.
- **The names `undefined`, `NaN` and `Infinity` count as their global values only when they
  provably are**. A reference must resolve to no binding (a parameter, `var`, `let`
  or `const` named `undefined` shadows it), must not sit inside a `with` statement, and the
  program must hold no direct `eval`; the check is `untwist_core::ast_util::global`. Passes that
  create the name check the same at the insertion point. Each refusal keeps the input as written,
  in default and in the conservative comparison mode, where the reference deobfuscator rewrites and changes what the program prints:
  `yoda` keeps `undefined === (undefined = 1)` (flipped, the write runs before the read);
  `nullish-coalescing`, `optional-chaining` and `default-parameters` do not treat a shadowed
  `undefined` as the undefined value (`a !== null && a !== undefined ? a : d` is not `a ?? d`
  there); `unwrap-comma-call` keeps `(undefined, g)()` when the prefix reads a lexical binding,
  which may throw in its temporal dead zone; `void-to-undefined` and `infinity` do not write
  `undefined`/`Infinity` inside a `with` statement or in a program with a direct `eval`. The same
  scope rule now covers `json-parse`'s `JSON` and `evaluate-globals`' `atob`, `unescape`,
  `decodeURI` and `decodeURIComponent`, which used to check declared bindings only.
- **`json-parse` and the array-callback arm of `remove-double-not` rely on built-ins the program has not replaced.**
  `JSON.parse("[1,2]")` is the literal only while `JSON.parse` is the built-in, and
  `xs.filter((v) => !!v)` may lose its `!!` only while `filter` (or `some`, `every`, `find`,
  `findIndex`, `findLast`, `findLastIndex`) coerces its callback's result. By default each asks
  the scan `untranspile` uses (`untwist_core::semantic::builtins`) at its first candidate and
  rewrites in the round after it, only when the program neither writes, deletes nor defines
  `JSON.parse` or one of those `Array.prototype` methods (also through a value such as
  `[].__proto__`, `x.constructor.prototype`, `Object.getPrototypeOf(x)`, `y.JSON` or `this[k]` in a
  non-strict function, whose `this` is the global object when it is called plainly), nor lets
  `JSON`, `Array`, `Array.prototype`, the global object or a prototype reached that way escape
  into a value, nor runs code from text (`Function(…)`, an indirect `eval`), in default and in the
  conservative comparison mode. `--unsafe` rewrites both without the scan, under the assumption
  that **built-ins are never replaced**. A `!!` in a test position (`if`, loops, `?:`, under `!`) is removed in every
  program, since no built-in is involved there. The reference deobfuscator rewrites all of
  these and changes what a program that patches the built-in prints.
- Class members named `constructor` (and static members named `prototype`) and object properties named `__proto__` are never un-computed by `computed-properties`; doing so would change semantics or be a syntax error.
- **A negated `if` whose consequent is a lone nested `if` is inverted by default so the chain can
  merge.** `if (!c) { if (d) {…} else {…} } else {…}` becomes `if (c) {…} else if (d) {…} else {…}`.
  The reference deobfuscator merges only the else-branch form (`unminify/transforms/merge-else-if.ts`); its own open PR
  #166 adds this inversion, closing issue #165, and it is unmerged upstream. The inversion fires only
  for a `!`-unary test (not `!=`), only when an `else` exists, and only when the consequent is a block
  holding exactly one `if` — the shape the inversion unlocks. Inverting every negated `if` would be a
  readability opinion rather than an unminification. The conservative comparison mode leaves the form alone. Multi-level
  flattening needs no such arm in either mode: `exit_statement` is bottom-up inside the fixpoint
  driver, so an arbitrarily deep chain collapses in one traversal where the reference deobfuscator needs the PR.
- **A `let` the program never writes becomes `const` by default.** The reference deobfuscator has no such pass; its own
  open PR #172 adds one and is unmerged. Untwist's runs last in the unminify group so every other pass
  sees `let` as the input spelled it, is gated by the conservative comparison mode, and splits a mixed
  `let a = 1, b = 2; b = 3;` into `const a = 1; let b = 2;`. Three shapes are deliberately refused: a
  declarator with no initialiser, a `for`/`for-in`/`for-of` head, and a destructuring pattern — the
  pattern binds several symbols and would need a second rule for when the declarator as a whole is
  constant. A program with a direct `eval` or a `with` statement is refused whole, with one note: the
  scoping tables cannot see a write those make, and `let x = 1; eval("x = 2")` runs where the `const`
  form throws. A top-level `let` is kept when the program is not an ES module: a classic
  script's top-level `let` is a global lexical binding that indirect eval (`(0, eval)("x = 2")`), the
  `Function` constructor, string timers and any later script can write, none of which the scoping
  tables can see, so `let x = 1; (0, eval)("x = 2")` runs where the `const` form throws — this is
  the real reason behind #172's top-level skip. In a module the top level converts, since module
  scope is closed to outside code. Module-ness is decided by syntax evidence — an `import` or an
  `export` statement in the program body (`untwist_core::semantic::has_module_syntax`) — never by
  the parser's source type: `parse_program` is module-first and calls every strict-mode-clean
  script a module, which would open the hazard on nearly every browser script. A Node CommonJS
  module (whose top level is function-wrapped) and a browser script both lack that evidence, so
  the skip applies to both.
- **`(0, f)(args)` becomes `f(args)` by default, and `(0, obj.f)(args)` never does.** Bundlers emit the
  comma form to sever `this`: `obj.f(x)` passes `obj` as the receiver and `(0, obj.f)(x)` passes
  `undefined`, so unwrapping a member callee changes what the program does. An identifier, a
  function-expression or an arrow callee has no receiver either way, so the unwrap is exact. The dropped prefix
  must be pure by `ast_util::literal::is_pure_value` or be `void 0`/`undefined` by
  `ast_util::literal::is_undefined_like` — `(log(), f)(x)` keeps its call. `(0, eval)(s)` is also kept:
  it is an indirect eval, and `eval(s)` would be a direct one with access to the local scope. A
  program with a direct `eval` or a `with` statement is refused whole with one note, as
  `let-to-const` refuses it: an identifier resolved through a `with` object's environment record
  carries that object as its receiver, so inside the block `(0, f)()` and `f()` differ. The reference deobfuscator
  has neither form (issues #120 and #6 are both open), so the conservative comparison mode leaves every shape alone. This
  closes both issues with one pass: #120 asked for the narrow JSX-adjacent case and #6 for the general
  one, and the general one minus the member case is the sound rule.
- **`x != null ? x : y` becomes `x ?? y`, in every mode.** The loose test is the form terser
  rewrites the strict pair `x !== null && x !== undefined` into, so refusing it would lose most real
  cases. The two differ for one value only: `document.all` (an object with the `[[IsHTMLDDA]]` slot)
  is loosely equal to `null` but is not nullish, so `document.all != null ? document.all : g` gives
  `g` and `document.all ?? g` gives `document.all`. Untwist assumes no tested value is
  `document.all`; the strict pair carries no such assumption. TypeScript emits the strict pair for
  this reason, and Babel's loose mode emits `!= null` under the same assumption.
- **`(_a = x) == null ? void 0 : _a.y` becomes `x?.y`, in every mode.** esbuild and Babel's loose
  mode lower optional chains with the loose test (`x == null ? void 0 : x.y` for a repeatable `x`
  too). It differs from `x?.y` for `document.all` only, which `?.` reads through; the same
  assumption as `x != null ? x : y` above.
