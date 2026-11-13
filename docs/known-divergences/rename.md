# Known divergences from the reference deobfuscator: Rename

Untwist's `--rename mangle` output is verified against the reference deobfuscator's `-m` mangle by `cargo xtask
diff-mangle`; the items below are deliberate, not bugs. `--rename simple` and
`--rename unique` have no counterpart in the reference deobfuscator.

- **Self-exclusion.** The oracle lets a symbol keep the name it held on entry, so
  mangling an already mangled program is a no-op. The reference deobfuscator's `-m` mangle run twice turns `vLN1` into
  `vLN12`, because `generateUid` sees `vLN1` bound — by the very binding it is renaming
  (`ast-utils/scope.ts:15`, `ast-utils/rename.ts:67-69`). By default the oracle goes one step
  further: a candidate is refused only when it is a label, was handed out this run,
  or is held by something the run does not rename — a global reference, a pinned binding or a
  binding whose role infers no name — so a name held only by the *original* name of a binding
  this run renames is free, and `var f = 1; function g() { return f; }` prints
  `var vLN1 = 1; function f() {…}` on the first run and again on the second. The reference deobfuscator's `-m` mangle
  refuses every name in `program.references`, which holds every binding's original name, and
  prints `f2`; the conservative comparison mode keeps that rule byte for byte. A binding that exhausts its 999
  candidates under the default rule could keep a name another binding was handed, so mangle
  then leaves the whole program unrenamed with one note, as `simple` and `unique` do. Idempotency (renaming an
  already renamed output changes nothing) is measured over the three `diff-mangle` corpora in all three modes by `rename_acceptance`
  (`renaming_an_already_renamed_output_changes_nothing`) with an empty
  `crates/untwist-cli/tests/rename-idempotency-allowlist.txt`. By
  default the rename stage runs last, after protections, JSX, cleanup, the settle loop, the
  final machinery sweep and the lexical stage, so no later stage deletes a binding whose name
  rename had spent or changes a binding's scope or declaration shape after it was named.
  The conservative comparison mode keeps the reference deobfuscator's order (rename after unminify, before protections, JSX and
  cleanup, and again after a final machinery sweep that changed something) and is not
  idempotent there, as the reference deobfuscator is not. Because JSX now converts before rename, a binding used
  as a JSX element name keeps an initial capital in every mode: the oracle upper-cases the
  first letter of each candidate for it (`simple` `c` → `C`, mangle `f2` → `F2`, unique
  `fn1` → `Fn1`), so a component never turns into an intrinsic element; this also covers JSX
  written in the input, in the conservative comparison mode too (the reference deobfuscator's `-m` mangle would lower-case it). Under
  `unique` the capitalised name is a different string from the counter's lower-case one, so
  cross-file uniqueness of such a name rests on the counter alone. A binding declared more than once
  is decided at every declaration site, the way the reference deobfuscator renames it at every
  `BindingIdentifier` exit: each earlier site's name is handed out and never released
  (`scope.ts:21-22`), so `function f(a, b) { var [a, b] = z }` costs `p` and `p2` before
  settling on `v` and `v2`, the next parameter anywhere is `p3` on both sides, and the final
  name is chosen at the last site. Refusing the original name is the one thing Untwist does
  not reproduce at those later sites: the reference deobfuscator's second visit finds the original name in `program.references` and
  refuses it, so `var vLN12 = 1; var vLN12 = 1;` becomes `vLN14` under the reference deobfuscator's `-m` mangle and stays
  `vLN12` under Untwist. Idempotency also needs a variable's derived name to read the name its source
  *receives* rather than the name it had — the **Derived names** item below.
- **Derived names.** By default a variable's mangled name derives from the name
  the binding it reads receives in the same run: `var named = function inner() {}` prints
  `var vF = function f() {}`, `var user = getUser(); function getUser() {}` prints
  `var vF = f(); function f() {}`, and `var a = b; var b = 1;` prints `var vVLN1 = vLN1;`, so
  mangling the output again changes nothing. The binding a variable reads — an
  identifier initialiser, the callee at the bottom of a `Call`/`require` chain, or a named
  function/class expression's own name — is decided before the variable, recursively for a
  variable that reads a variable, to a depth of four; a read that would close a cycle
  (`var a = b; var b = a;`) or exceed the depth contributes no suffix (`var vV = v; var v =
  vV;`), which is the only rule that both terminates and is stable on its own output.
  the reference deobfuscator's `-m` mangle reads the still-original name, because Babel exits the declarator's `id`
  before entering its `init` (`vInner`, `vGetUser`, `vB`), and only reaches `vF` on its
  second run, alongside the `f2` of the self-exclusion item above. The conservative comparison mode reproduces that first run
  byte for byte and gives up idempotency on these shapes, exactly as the reference deobfuscator does.
- **Export pinning.** Export-pinned bindings are never renamed in any mode: a binding
  an `export` declaration itself declares, or an `ExportSpecifier` local whose local and
  exported names are equal. `export { x as y }` is still renamed, because oxc prints
  `local as exported` whenever the names differ; the reference deobfuscator skips it either way, because Babel
  records the `ExportNamedDeclaration` as a reference regardless of aliasing.
- **The 999-candidate cap.** `uid_candidates` stops at `base999`; mangle then leaves the
  whole program unrenamed with one note (`ran out of names for …`), as `simple` and
  `unique` do. The reference deobfuscator's `generateUid` loop is unbounded. Untwist fails closed: it never
  hands out a name it has not checked.
- **`toIdentifier` per candidate.** The reference deobfuscator applies `toIdentifier` to every candidate in
  the loop (`ast-utils/scope.ts:11`), so `require("node:fs")` twice gives `nodeFs` then
  `nodeFs2`. Untwist applies `to_identifier` once to the base and appends the suffix. The
  two agree for every identifier base, which is every base Untwist builds; the difference is
  only reachable for a base that is not already an identifier.
- **The refined descendant safety check.** The oracle also rejects a candidate name
  bound in a **descendant** scope of the binding being renamed, when the binding is actually
  referenced inside that scope. The reference deobfuscator's `generateUid` only walks *up* the scope chain, so
  it can hand a name to an outer binding that an inner scope already shadows and silently
  capture the outer binding's own references. Untwist refuses and takes the next suffix,
  which can show up as a `p2` where the reference deobfuscator prints `p`.
- **Annex B.3.3 block-level function declarations are pinned.** In sloppy
  code a `function` declaration inside a block, a `case` body or an `if` clause is hoisted as
  a `var` in the enclosing function, so `function f(){ {function g(){return 7}} return g() }`
  returns `7`. oxc binds `g` in the inner scope, which makes the outer `g()` an unresolved
  reference, so renaming the declaration strands the call. The reference deobfuscator's `-m` mangle emits byte-identical
  broken output — a `ReferenceError` program — and Untwist used to match it. Untwist now pins
  such a declaration in every mode and reserves its name program-wide, because Annex B would
  otherwise assign the block function to any enclosing-scope binding a rename gave that name
  on block entry. Strictness is decided from the source rather than from oxc's `ScopeFlags`:
  `parse_program` tries module syntax first, so an ambiguous script is reported strict.
  Untwist treats a program as strict only when it carries a `"use strict"` directive or real
  module syntax, and inherits strictness through functions, arrows and class bodies; anything
  else is assumed to be sloppy, where pinning is the safe answer.
- **Shorthand destructuring-assignment targets are pinned.** In
  `({ a } = o)` the identifier is both the property key and the assignment target — oxc's
  `AssignmentTargetPropertyIdentifier` stores it once, as `binding`, and the ESTree
  serialisation names that same node both `key` and `value`. Its printer
  (`oxc_codegen-0.148.0/src/gen.rs:2202-2213`) can only emit the colon form when a *mangler*
  supplies a name different from the AST one; Untwist rewrites the AST in place, so a rename
  is printed back as shorthand under the **new** name and the assignment silently reads a
  different property: `let a = 0; const o = { a: 1 }; ({ a } = o)` returns `1` before and
  `undefined` after. The reference deobfuscator's `-m` mangle emits byte-identical broken output — it renames the
  identifier and so renames the property read — so this is a shared miscompile rather than a
  Untwist regression, but faithfully matching a miscompile is still a miscompile. Untwist
  pins such a binding in every mode. The colon form `({ a: b } = o)` carries its own key node
  and is unaffected: `b` is renamed freely, and oxc omits the key only when it equals the
  target name (`omit_key`, same lines), so a collapse to `({ a } = o)` can never change which
  property is read.
- **Shorthand restoration is an untwist-only stage, disabled in the conservative comparison mode.** Destructuring, object-literal and import-specifier shorthand is restored by
  renaming the local onto its key (`rename-shorthand` stage), not by setting an AST flag:
  oxc's printer recomputes shorthand from key/name equality for every key **except**
  `__proto__` (`oxc_codegen-0.148.0/src/gen.rs:1795-1804`), where `ObjectProperty` honours
  the AST flag verbatim and never compares it with the value. `BindingProperty`
  (`gen.rs:3119-3137`) has no such exception and always recomputes. That exception matters
  because the two spellings differ in meaning: `{ __proto__: v }` sets the literal's
  [[Prototype]] while `{ __proto__ }` creates an own property named `__proto__`, so no rename
  of the value can preserve the shorthand spelling. So a binding used as the
  value of a `{ __proto__ }` shorthand is pinned in every mode, and the stage skips
  `__proto__` keys in both directions — it never renames a binding onto that key and never
  collapses `{ __proto__: v }`. The reference deobfuscator has no counterpart stage, so `Shorthand` returns
  `true` from `requires_extended()` and the conservative comparison mode filters it out; `cargo xtask
  diff-mangle` therefore compares mangle against mangle. mangle itself matches the reference deobfuscator: it
  never renames a destructuring-pattern or import-specifier binding, matching the reference deobfuscator's
  early returns at `transforms/mangle.ts:16-17`.
  `fixtures/unminify/destructuring-shorthand.js` exercises all three shapes and is
  byte-identical to the reference deobfuscator on both differentials.
- **Unminify divergences reach this differential too.** `cargo xtask diff-mangle` compares
  whole-program output with the unminify stage enabled on both sides, so a fixture that
  differs from the reference deobfuscator for a non-rename reason is on `xtask/diff-mangle-allowlist.txt` with
  that reason, whether or not `diff-unminify` also compares it. Four fixtures are there today,
  and the mangled names match the reference deobfuscator exactly on every one of them:
  `fixtures/unminify/strings-templates.js`, where only the `Safety::Unsafe`
  `template-literals` pass differs, and `fixtures/reference/deobfuscate/obfuscator.io.js`,
  `fixtures/reference/deobfuscate/obfuscator.io-var-functions.js` and
  `fixtures/reference/unpack/browserify-cocos2d.js`, where oxc_codegen keeps the parentheses
  of an IIFE that the reference deobfuscator's printer drops (the item at the top of this file).

One allowlist entry is a reference-side artifact rather than a Untwist choice:

- **A parameter the reference deobfuscator's own transpile stage synthesised is still renamed.**
  `transpile/transforms/default-parameters.ts:152-153` moves an identifier out of a `var`
  declarator into `fn.params` by node mutation and then removes the declaration, without
  re-registering the binding with Babel's scope. The later `mangle` pass therefore finds no
  binding for it (`transforms/mangle.ts:20-21`, `if (!binding) return`) and leaves the
  parameter under its minified name. Untwist re-derives scopes before renaming and names it
  `p`, which is what the reference deobfuscator's `-m` mangle itself produces when handed the same unminified code
  directly. Untwist is simply not reproducing a defect in the reference deobfuscator's
  own bookkeeping. This is the whole of the `x` versus `p` difference on
  `fixtures/unminify/transpiled-babel.js`.

Not divergences from the reference deobfuscator, but worth recording about the other modes:

- `Simple` may reuse a name that an enclosing scope binds when the renamed scope never
  references that outer binding, which is what lets every function restart
  its sequence at `a`. The alpha-equivalence property test in
  `crates/untwist-rename/tests/` proves each such reuse is capture-free.
- `Unique` keeps its counters **per base name**, program-wide and shared across every input
  of one run (`fn1`, `fn2`, `arr1`, `arr2`), rather than one global declaration-order
  counter. Names stay globally unique because `(base, number)` pairs are, and adding an
  unrelated binding does not reflow the numbers of everything after it, the problem
  the reference deobfuscator's mangle has. A base
  derived from a call or construction reads the callee's original name when that name looks
  human-written and does not end in a digit (`var user = getUser(); function
  getUser() {}` is `user1`/`fn1`), and otherwise the base the callee receives in this run
  (`var a = b(); function b() {}` is `fn1`/`fn2`). A renamed callee such as
  `fn1` fails that test, so on a second run the original name is gone; the binding's own name
  then keeps its base when it is a human-looking stem plus a number (`user1` reads `user`) and
  is not already the received base plus a number. That is what keeps `user1` from turning
  into `fn2` on the second run. A callee that this run leaves alone always gives its own
  name. `fixtures/unminify/rename-derived-names.js` holds the shapes and `rename_acceptance`
  proves all three modes are no-ops on their own output over it.
- **Class-identity hints** (`Unique` only). A class, or a function or variable
  holding a class or function, takes its base from the first of: (a) `this.type =
  "<Name>"` in its constructor; (b) `this.is<Name> = true` there, or `C.prototype.is<Name> =
  true`; (c) a static `name`/`displayName` string field (`static displayName = "X"`,
  `C.displayName = "X"`, `C.prototype.name = "X"`). `<Name>` must be ASCII PascalCase with a
  lowercase letter, and must pass the homoglyph guard (`looks_human_written`). Inside the
  constructor `this` also covers an alias assigned from `this`, from a call whose first
  argument is `this`, or from `x || this`, which are the transpiled `_this = _callSuper(this,
  C)` and `_this = _super.call(this) || this` forms. A string key (`this["isMesh"] = !0`)
  counts, and so does a template literal with no substitutions. Arrow functions keep the
  constructor's `this`; other nested functions and methods do not. The hint applies when the
  class's original name does not look human-written, or when that name is the hint followed
  by digits (`Mesh1`, its own output). A human-named class keeps its usual base under
  `rename_all`. Constructions read the hint as a received base, so `new _0xa()` with `this.type
  = "Mesh"` is `mesh1`. On game.js with `--rename unique`, 152 bindings get a hint name in
  the default pipeline (`Mesh2`, `WebGLRenderer2`, `PerspectiveCamera2`, …; the `1`s go to the
  outer `var` that shorthand restoration then names `Mesh`) and 32 without deobfuscation or
  unminify. Without a hint nothing changes. `mangle` and `simple` do not read hints.
- **Component names the JSX stage invents** (`Unique` only). The JSX stage renames a
  lower-case component binding to `Component` (`Component_2__`, …; `_Component` under
  the conservative comparison mode). Under `rename_all`, `Unique` reads such a name on a binding used as a JSX tag as
  the base `Component` (`Component1`, `Component2`, idempotent), where it used to print
  `Fn1`. `--keep-human-names` already keeps the invented names in `simple` and `unique`.
  `simple` under `rename_all` still prints its sequential capitals (`A`), and mangle stays at
  the conservative comparison mode (`F`). The
  CommonJS/AMD wrapper names `require`, `module`, `exports` and `define` are exempt from
  `rename_all` in `Simple` and `Unique`: renaming a wrapper's `require` to `cb1`
  turned every `require("./sum")` base into a callee-derived `cb2` on the next run.
- **Function and class names.** Renaming a binding that gives a function or class its name
  changes what `.name` returns. So in the safe lane, when the file reads `name` with a static key
  (`f.name`, `f["name"]`, `const { name } = f`, `({ name } = f)`) on a value that is not an
  object, array or literal, every such binding keeps its name in every mode: function and class
  declarations, named function and class expressions, and a variable, parameter default or
  assignment whose value is an anonymous function, arrow or class. A file with no such read is
  renamed as before, and `--unsafe` renames them anyway. The limit, like `fn.toString()`: a read
  through a key that is not a literal (`f[key]`) or through reflection (`Reflect.get(f, "name")`,
  `Object.getOwnPropertyDescriptor`) is not seen. The host also reads `name` when it prints a
  value: `console.log(f)`, `console.dir`, `util.inspect` and the devtools show
  `[Function: newName]` or `NewClass { … }` for a function, a class or an instance. Like error
  stack traces, what the host prints is outside the promise, so the guard does not look for it.
- **Function source under `smart`.** `smart` keeps every name inside or referenced by a function
  whose source the program reads (`'' + f`, `` `${f}` ``, `String(f)`, `f.toString()`), and leaves
  the whole program unrenamed when a function may read its own source through a method call on
  `this`, `arguments.callee` or `.caller` (`this.toString()`); other routes are text-only reads
  (below). With `--unsafe`, `this.toString()` inside an instance method (a
  function stored on `X.prototype`, other than `Function.prototype` and `Object.prototype`, or a
  non-static class method) is taken to read the instance, not a function, so it does not stop
  renaming: a call that hands such a method a function as its receiver
  (`Number.prototype.m.call(f)`) would see renamed source.
- **Hoisting and the host's global object.** A sloppy block function and a `var` or parameter
  of the same name in its function are one binding to Annex B, so both keep their names. A `var`
  written inside a block or catch body counts where it hoists to, so no block-scoped binding in
  that body takes its name and it takes none of theirs. In a classic script, every top-level
  `var` and function keeps its name in every mode and both lanes when the file may read it off
  the global object (`window.x`, `globalThis[k]`, `this.f()` at the top level, indirect `eval`)
  or may observe the global object's names (`"x" in globalThis`, `hasOwnProperty`,
  `Object.hasOwn`, `Object.keys`, `getOwnPropertyNames`, `Reflect.ownKeys`, `for…in`,
  `getOwnPropertyDescriptor`); a key that is not a literal keeps them all. When several classic
  scripts are given together they share one global scope, so no top-level binding of a classic
  script is renamed in that run, and no top-level binding of any input takes a name another input
  binds at the top level or reads free. In a classic script, shorthand
  restoration leaves top-level `var` and function bindings alone (each is a property of the
  global object), and no mode gives a top-level binding the name of a browser or Node global
  property (`status`, `location`, `top`, `name`, `onload`, ...). When code built from text can
  reach the script's globals, top-level `let`, `const` and `class` keep their names too, since
  indirect `eval` and `Function` resolve them.
- **CommonJS wrapper names** (`name-commonjs-wrappers`, on in every mode and both lanes). It
  renames only the bindings of a recognised wrapper and only when no code can see the old
  name: every function whose text holds one of them is an IIFE, a discarded function
  expression, a function declared or held by a `var` that is only ever called directly (in a
  classic script, not at the top level), or a callback handed to a recognised
  `__commonJSMin`; none of those functions reads its own `arguments`; and in sloppy code no
  `.caller` read exists other than one compared with `==`/`===`. A direct `eval` or `with`
  anywhere turns it off. The wrapper function itself is renamed past the safe lane's `.name`
  guard only on those terms, since its value never leaves its call sites. Error stack traces
  show the new names, and a read through a computed key that is not a literal (`f[key]` that
  is `caller` at run time) is not seen, as for the rename modes.
- **Text-only reads.** Names a function's source shows after renaming (a function stringified
  through a parameter, `[f].join()`, `Reflect.apply(Function.prototype.toString, f, [])`, a
  wrapper inside a named IIFE, a class static block or a stringifying `__commonJS` look-alike)
  fall under the printer premise in [codegen.md](codegen.md) ("Function source text is not
  kept"): the `smart` source-text pin and the wrapper pass's sealing are best effort for such
  reads, not a promise.
- A program containing a direct `eval` call or a `with` statement is renamed anyway, with a
  diagnostic note naming the hazard. Static scope analysis cannot see what those constructs
  resolve; the reference deobfuscator reports nothing at all.
- **`cargo xtask diff-mangle` compares 50 fixtures**: `fixtures/unminify`, `fixtures/reference/deobfuscate`
  and `fixtures/reference/unpack`, all with the deobfuscate, unpack and JSX stages off on both sides, so
  the rename stage is the only stage that differs from the unminify differential.
  `fixtures/real-world/game.js` is deliberately excluded — it is 10.7 MB
  and `diff-deobfuscate` already carries it when present — and `fixtures/eval` is generated and gitignored, so a
  differential over it would not reproduce from a clean checkout; `crates/untwist-eval/tests/corpus.rs`
  covers that corpus for totality instead. Every allowlist entry names the rule above that owns it.
- Covered here: import-path-derived names, shorthand restoration, diff-stable `Unique`
  numbering, object-literal-value base names and the non-ASCII round-trip.

