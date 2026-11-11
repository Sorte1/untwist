# Known divergences: Untranspile

The untranspile passes turn transpiler output back into the syntax it was compiled
from: TypeScript, Babel, esbuild and SWC downlevel helpers, destructuring, `var` and `function`
forms. The reference deobfuscator has no counterpart, so every pass is outside the conservative comparison mode and the conservative comparison mode runs none of
them. A rewrite that is exact only under a named assumption about the program's runtime data or
callers is an `--unsafe` rewrite, and each such assumption gets a bullet below.

## `untranspile-helpers` (the helper framework and the spread helpers)

- **What counts as a helper.** A function declaration, or a `var` initialised with a function,
  an arrow, TypeScript's `(this && this.X) || function` (only at the top of the program) or one
  of esbuild's `Object.*` aliases, is a helper when its body's structural fingerprint (every node,
  operator, property key and literal; names, parentheses and the `!0`/`!1`/`void 0` spellings
  ignored) equals a known version in `crates/untwist-unminify/src/untranspile/shapes/`, every
  helper it calls is itself an identified helper of the expected kind, its binding is written
  only by its own body and not redeclared, and the program has no direct `eval` or `with`. Call
  sites resolve by symbol, so a shadowing local of the same name is never the helper. Each known
  version names the fixture that produces it. A helper named like a known one whose body matches
  no version is left in place with one note. Helpers are recognised in the traversal that reaches
  their definition, so a call site that runs before the definition in program order stays as it
  is.
- **Built-ins.** Every rewrite relies on the built-ins the helpers and the spread syntax use.
  The first round after a helper is recognised scans the program once: a write, `delete` or
  `Object.defineProperty`-style definition on `Object`, `Array`, `Symbol`, `Function`, `String`,
  one of their prototypes or the global object (restricted, on `Array`, `String` and `Function`
  and their prototypes, to the members the helpers call) keeps every helper, with one note.
  `String.prototype.escape = …` does not count; `Array.prototype.concat = …` does. If one of
  those objects escapes into a value (`var root = globalThis`, `var proto = Array.prototype`,
  `f([].constructor)`, `var p = x.__proto__`), is written through a value that may reach it (a
  write through `x.__proto__`, `x.constructor`, `x.constructor.prototype`,
  `Object.getPrototypeOf(x)`, `Reflect.getPrototypeOf(x)` or `Reflect.get(x, k)`, a write of a
  relied-on name such as `x.Array = …` on any object, or a definer call on one of those values),
  or the program runs code from text (`Function(…)`, an indirect `eval`, `setTimeout` with
  anything but a function, `vm.runInThisContext`),
  only `--unsafe` rewrites, under the assumption that **an aliased built-in object, or one
  reached through a value, is never modified, and code run from text modifies none**. The scan and the rewrites happen in the round after the helpers are recognised;
  when nothing else in the program changed, the pass asks the driver for that round itself, so
  a program that is already formatted still converts
  (`fixtures/untranspile/hand/formatted-spread.js`).
- **Safe rewrites** (default and `--unsafe`): `__spreadArray(to, [ … ], pack)` with array
  literals (`pack` false keeps holes as elisions; `pack` true refuses a literal with holes);
  `_toConsumableArray` of an array literal without holes, a string or a template literal;
  `[ … ].concat([ … ], …)` with only array literals; object spread helpers whose sources are
  `null`, `undefined`, strings or object literals of data members (inlined), with Babel's even
  positions, esbuild's `__spreadProps` and SWC's `_object_spread_props` inlining the literal's
  members as written and Babel's `_defineProperty({}, k, v)` chains becoming computed members
  when `k` is a literal or `v` cannot run code. A literal member `__proto__: v`, a member that
  uses `super`, or (for `__assign`) an accessor or a possibly-`__proto__` key refuses. In one call
  with several sources, a source after one whose copy can run code must itself be inert.
- **`--unsafe` rewrites**, one assumption each:
  - Babel and SWC `_toConsumableArray(x)` → `[...x]`: **the value is iterable or an array read
    in index order** — no array has its own or a non-default `Symbol.iterator`, is a proxy or
    changes its length while copied, and no non-iterable passed has a `"@@iterator"` key or an
    `Arguments`, typed-array, `Map` or `Set` tag (the helper copies those by index, spread
    throws).
  - TypeScript `__spreadArray(to, x, true)` → `[...to, ...x]`: **the spread value is an array**
    (the helper reads `length` and indexes; spread iterates). A literal that is never an array
    (a string, `undefined`, `new …`) refuses. `fixtures/untranspile/ts/generators.es5{,.min}.js`
    spread generator objects, print differently after the rewrite, and are the assumption's
    witness on `xtask/diff-behaviour-allowlist.txt` for the default lane.
  - TypeScript `__spreadArray(to, __read(x), false)` → `[...to, ...x]`: **the value is iterable
    and its iterator's `next` and its results' `value` never throw** (`__read` then calls
    `return`, spread does not; `__read` returns a non-iterable unchanged).
  - TypeScript `__assign` (`Object.assign`) with a source that is not a literal: **no source has
    an own `__proto__` property** (`Object.assign` sets the target's prototype through the
    inherited setter, spread defines an own property). `fixtures/untranspile/hand/assign-proto-source.js`
    is the witness: it matches in the default lane and is allowlisted in the `--unsafe` lanes.
  - Babel `_objectSpread2`, SWC `_object_spread` and esbuild `__spreadValues` with a source that is
    not a literal: **no source is a proxy or has a getter that adds, deletes or changes the
    enumerability of its own properties while it is copied** (the helpers list the keys first;
    spread checks each one as it copies).
  - Removing a helper defined at the top level of a script: **the program never reads a removed
    helper through the global object**. Helpers inside a function or a module are removed in the
    default mode once nothing references them.
- `__spreadArray(to, x, false)` with a non-literal `x` (TypeScript's argument spread), `__spread`,
  Babel's `_extends`, TypeScript's `es2015` `Object.assign` calls and `.apply(thisArg, [...x])`
  call spreads are not rewritten: the first keeps holes that spread would fill, and the others
  have no generated fixture yet.

## `var-to-let` and `function-to-arrow` (the lexical stage)

- **`function-to-arrow`, a callback argument (`--unsafe` only).** An anonymous function expression
  passed straight as an argument of a call becomes an arrow when it uses no `this`, `arguments`,
  `new.target` or `super` of its own, is not a generator and has distinct parameters. The rewrite
  assumes the callee never constructs the callback, never reads its `prototype` and never lists its
  own keys: an arrow is not constructible and has no `prototype`, `arguments` or `caller` own
  property. Default and safe modes leave such callbacks alone; a function expression bound to a
  constant that is only ever called directly (`f(…)`) becomes an arrow in every mode outside the conservative comparison mode,
  since nothing can tell.

## Destructuring

- **Array destructuring helpers.** `var t = _slicedToArray(x, n)` (Babel), `_sliced_to_array`
  (SWC) or `__read(x, n)` (TypeScript), followed directly by declarations `var a = t[i]` of the
  same kind, in increasing index order below `n`, that are the temporary's only references,
  become `var [a, , b] = x` with `n` slots (the trailing elisions step the iterator as far as the
  helper does). A read anywhere else, an assignment instead of a declaration, a default or a
  nested pattern keeps the helper call. The rewrite is Safe when `x` is an array literal or a
  string literal; otherwise it is an `--unsafe` rewrite:
  - Babel and SWC: **the value is iterable or an array read in index order** — the same
    assumption as `_toConsumableArray` above (an array is read by index, not iterated; a
    non-iterable with an `Arguments`, typed-array, `Map` or `Set` tag is copied, where
    destructuring throws), and an iterator's `return`, when called, returns an object.
  - TypeScript `__read`: **the value is iterable and its iterator's `next` and its results'
    `value` never throw** (`__read` returns a non-iterable unchanged, and closes the iterator
    after a throwing `next`; destructuring throws, and does not close).
- **Object rest helpers are kept**, with one note per kind and program: Babel and SWC
  `_objectWithoutProperties` and `_objectWithoutPropertiesLoose`, TypeScript `__rest` and
  esbuild `__objRest` copy with `[[Set]]` (an own `__proto__` key sets the prototype), list the
  keys before copying, return `{}` for `null`, and (the loose form) omit symbol keys; object rest
  does none of these.

## `merge-member-declarations` (reference deobfuscator issue #30)

- Consecutive declarations `var a = e.x; var b = e["y"];` of the same kind, each with one
  binding and a non-optional static read of the same identifier `e`, become
  `var { x: a, "y": b } = e;` in every mode outside the conservative comparison mode. The pass runs last in the lexical
  stage, so the kinds are already the ones `var-to-let` chose. It is exact: `e` is a constant
  binding (never written, not redeclared), the reads keep their order, and destructuring reads
  the same keys in the same order and throws a `TypeError` for `null` or `undefined` as the reads
  do. A run stops at a repeated binding or a binding named `e`; `e` must not be a parameter or
  `var` of a function that reads `arguments` (the arguments object can rebind it between the
  reads), `e` must not be imported or be a `var`, `let`, `class` or function at the top of a
  script (the global object, another script or an indirect `eval` can replace those; a top-level
  `const` merges), and a program with a direct `eval` or `with` merges nothing.

## `inline-object-props`, variable-key reads

- **An unknown-key read keeps the map but not its static reads (`--unsafe` only).** Beyond
  the conservative comparison mode, the cleanup registration of `inline-object-props` classifies a computed read `O[k]` of
  a literal-valued map whose key is not a literal (rules in `deobfuscate.md`, "Outside the conservative comparison mode,
  `inline-object-props` reads through a computed key that is not a literal"). A key it cannot
  resolve, read in a value position, keeps the map's declaration; under `--unsafe` the map's
  static and resolved reads are still inlined, under the assumption that **no accessor the
  program installs on `Object.prototype` runs when the map is read with a key it does not own**
  (such a getter receives the map as `this` and could write it).
  `fixtures/behaviour/object-inherited-getter.js` is the witness. A key that is a constant local
  binding initialised to a string or numeric literal before the read resolves exactly and is
  inlined in the default mode.

## Classes

- **What becomes a class.** Babel's class shapes become a class expression in place of the
  helper call, so the binding keeps its `var` (its hoisting, and its global-object property at
  script level, are unchanged; the lexical stage then applies its own rules):
  `var C = function ([_super]) { function C(…) { _classCallCheck(this, C); … } [_inherits(C,
  _super);] return _createClass(C, [protos], [statics]); }([P])` and
  `_createClass(function C() { … }, [protos], [statics])` become `class C [extends P] { … }`.
  Every helper involved must be an identified version (`_classCallCheck`, `_createClass` with
  `_defineProperties`, `_inherits` with `_setPrototypeOf`, `_callSuper` with its parts, and
  `_defineProperty` or esbuild's `__publicField` for fields), including terser's copies with the
  single-use helpers inlined. The constructor and every method must be strict code (a
  `"use strict"` directive on the program, an enclosing function, the IIFE or the function
  itself, or module code), the class binding must never be written, and the IIFE must hold
  nothing but those statements. Property keys must be string or numeric literals or a
  `Symbol.*` member: anything else (a key that reads the class, a computed expression) keeps the
  class. An instance key `constructor` and a static `prototype`, `name`, `length`, `caller` or
  `arguments` keep it too.
- **Constructors.** `_classCallCheck` goes. A derived constructor's `_this = _callSuper(this, C,
  [args])` becomes `super(args)` (`arguments` and Babel's `[].concat(rest)` are spread), and every
  later `_this` becomes `this`; it may also stand first in a member read, a call or a declaration
  (`(_this = _callSuper(…)).x = 1` becomes `super(…); this.x = 1;`). The class is kept when
  `_this` is read before the super call or inside a nested `function`, when `this` is used
  directly anywhere but the two helper calls, or when a constructor-level `return` returns
  anything but `_this`. A constructor that only forwards `arguments`, or a base constructor left
  empty with no parameters, is dropped (the default constructor does the same).
- **Fields.** The run of `_defineProperty(this, k, v)` / `__publicField(this, k, v)` calls that
  starts right after `_classCallCheck` (base) or right after `super(…)` (derived, plain
  assignment form only) becomes class fields `k = v`, in order; both sides define the property
  with `[[DefineOwnProperty]]`. A value that reads a parameter or local of the constructor,
  `arguments` or `new.target` ends the run, and so does a key that is not a literal or
  `Symbol.*` member. A base class moves fields only when its parameters are plain identifiers
  and nothing but declarations of `this` aliases or of uninitialised bindings precedes
  `_classCallCheck` (fields are initialised before the parameters and the body run). The alias
  (`var _this = this`) becomes `this` inside a moved value. Static fields stay after the class.
- **Every class rewrite is `--unsafe` only**, under these assumptions:
  - **Methods and accessors are never constructed, and their `prototype` and `name` are never
    read.** A function in a `_createClass` descriptor is constructible and has a `prototype`; a
    method is not and has none. Its `name` is the function's own (`"value"`, `"get"`, `""` when
    minified, the key only when Babel named it), where a method's is its key.
    `fixtures/behaviour/class-method-constructed.js` and the `classes-identity` fixtures are the
    witnesses.
  - **A class constructor is only ever constructed** (`new`, `super`, `Reflect.construct`).
    `_classCallCheck` throws only when `this` is not already an instance, so
    `C.call(instance)` runs the constructor; a class throws.
    `fixtures/behaviour/class-called-with-instance.js` is the witness.
  - **A derived class's prototype keeps `constructor` pointing at the class, and `Reflect` is
    the built-in.** `_callSuper` constructs the parent with
    `Object.getPrototypeOf(this).constructor` as `new.target` (or through `[[Call]]` when
    `Reflect.construct` is missing); `super(…)` uses the real `new.target`.
    `fixtures/behaviour/class-new-target-constructor.js` is the witness.
- **SWC classes convert only when nothing reaches their `prototype` property.** SWC's
  `_create_class` and `_inherits` leave `prototype` writable, which a class does not. Under
  `--unsafe` an SWC class converts, by the rules above, when a whole-program fact gathered once
  per round shows that nothing writes or inspects that property. The class binding (the
  constructor inside the IIFE, and the variable the IIFE's result is stored in) may only be:
  - a call or `new` callee;
  - the object of a member read, or of a member write under a literal key other than `prototype`;
  - an operand of `typeof`, `void`, `==`, `===`, `instanceof` or `in`;
  - an argument the class helpers take: `_class_call_check`, `_create_class` with literal static
    keys, `_inherits` once as the child and freely as the parent, `_call_super`,
    `_get_prototype_of`, `_type_of`, or a field definer with a literal key;
  - an IIFE argument whose parameter follows the same rules.

  Anything else keeps the class, with the note: a write or `delete` of `C.prototype` or
  `C["prototype"]`, a computed write `C[k] = v`, or `C` stored, returned, spread or passed to any
  other function (`Object.defineProperty`, `Object.getOwnPropertyDescriptors` and
  `Object.setPrototypeOf` included). Three program-wide signs keep every SWC class: a `prototype`
  write on anything but a plain binding (`this.prototype = …`, `x.constructor.prototype = …`), a
  `"prototype"` string passed to a call, and an object literal with a `prototype` key. The
  `hand/class-proto-*.js` fixtures are the witnesses, one per rule. **Still assumed:** no code
  that reaches the class through `this` or `.constructor` lists its property attributes
  (`Object.getOwnPropertyDescriptors`) or writes it under a computed key. SWC's helper-less form
  (`var C = function C() { _class_call_check(this, C); }`) is not rewritten.
- **TypeScript (tslib) classes are kept, with the same note**. tslib's `/** @class */
  (function (_super) { __extends(C, _super); function C() {…} C.prototype.m = function () {…};
  return C; }(P))` builds a plain function whose `prototype` stays writable, and a class's does
  not; nothing here proves the program never writes it (the binding is often script-global,
  where another script can). `__extends` is identified (`ts/classes.es5.js` and its minified
  copy), and while it is in the program `--unsafe` adds the note once. The helper-less base form
  is kept for the same reason. The prototype fact of the SWC bullet above is not enough here:
  there is no tslib class rewrite, and tslib assigns its methods (`C.prototype.m = function`), so
  they are enumerable where a class's methods are not.
- **esbuild native classes: `__publicField(this, k, v)` calls become fields**. In a
  native `class` (esbuild `es2015`–`es2019`, or Babel's `_defineProperty` in a native class),
  the run of field-definer calls that opens a base constructor, or that follows a leading
  `super(…)` statement in a derived one, becomes fields `k = v` with the rules of the "Fields"
  bullet above (literal or `Symbol.*` keys, values that read no constructor binding,
  `arguments` or `new.target`; a base constructor with plain parameters only, since fields run
  before the parameters bind). The fields go after any instance field the class already has
  (they are initialised in element order). A constructor left as `{}` (base) or
  `{ super(...arguments); }` (derived) with no parameters is dropped; one with parameters stays,
  so `C.length` does not change. Like every class rewrite it is `--unsafe` only. Witnesses:
  `fixtures/behaviour/class-native-param-order.js`, `class-native-body-first.js`,
  `class-native-field-order.js`, `class-native-ctor-length.js`.
- **Code the obfuscator moved out of strict mode stays.** game.js's bundle starts with
  `'use strict'`, but obfuscator.io puts statements before it, so the input is sloppy code and
  its classes are kept. Untwist's deobfuscation later removes those statements; the string then
  stays an inert expression statement, so the output is sloppy too (only the conservative comparison mode promotes it,
  as the reference deobfuscator does; see `deobfuscate.md`).

## Async functions and generators

- **Generators (tslib).** A function whose body is `return __generator(this, function (_a) {
  switch (_a.label) { … } })`, after only `var` declarations initialised with nothing, `this` or
  `arguments`, becomes a `function*` (TypeScript's `__generator`, SWC's `_ts_generator`; both
  identified by structure, minified copies included). The state machine is read case by case:
  `return [2, v]` returns, `[3, L]` jumps, `[4, v]` yields and resumes at the next case, whose
  first evaluated expression must be `_a.sent()`, `[5, __values(x)]` delegates (`yield* x`),
  `[7]` ends a `finally`, and `_a.trys.push([try, catch, finally, end])` opens a `try`. The
  cases are then structured (dominator-tree based, after Ramsey's "Beyond Relooper"): loops
  from back edges, `if`/`else` from branches, labelled `break`/`continue` where a jump crosses
  more than the innermost loop, and `try`/`catch`/`finally` from the try entries; a caught
  error read as `e = _a.sent()` becomes `catch (e)` when `e` is used nowhere else. A machine
  whose control flow is irreducible, that jumps into the middle of a `try`, reads the state
  object any other way, declares bindings in a split block, or sits in a getter or setter is
  kept with one note per function (`fixtures/untranspile/hand/machine-irreducible.js`,
  `machine-guards.js`).
- **Async functions (tslib and esbuild).** `return __awaiter(this, void 0, void 0, function* ()
  { … })` (TypeScript, also after the `es5` generator above is restored) and esbuild's
  `return __async(this, null, function* () { … })` become an `async` function (or an `async`
  arrow) whose body is the generator's, with each of its own `yield` turned into `await`. The
  outer function's parameters must be plain identifiers (an async function rejects where the
  wrapper throws). The receiver may be something other than `this` (TypeScript's `_this`,
  esbuild's `null`) only when the generator does not use `this`. When the wrapper forwards
  `arguments`, the generator's parameters replace the outer ones, which must be unread
  placeholders of the same count (`.length` stays). A generator binding whose name the outer
  function already binds, or a program that writes `Promise`, keeps the helper, with one note
  per function (`fixtures/untranspile/hand/async-guards.js`).
- **Every async and generator rewrite is `--unsafe` only**, under these assumptions:
  - **The program does not depend on how many microtask ticks an `await` or a resolution
    takes.** `__awaiter` adopts each awaited value with `new P(resolve => resolve(v)).then(…)`
    and esbuild with `Promise.resolve(v).then(…)`; native `await` and an async function's
    return take a different number of ticks, which interleaved async code can observe. The
    `async-identity` fixtures print such an order and are the witnesses (they are expected to
    diverge and are allowlisted, never pinned as equal).
  - **No code inspects a generator object's or a converted function's identity.** A tslib
    generator object is `Object.create(Iterator.prototype)` with own `next`, `throw` and
    `return`; a native one differs in its prototype chain, `Object.prototype.toString` and
    `instanceof`. The converted function itself is no longer constructible, and an async
    function has no `prototype` (`async-identity` prints `typeof one.prototype`).
  - **A `yield*` delegate is iterable.** `__values(x)` also walks an array-like `x` without
    `Symbol.iterator`; native `yield*` throws `TypeError` for it.
- **Reads that precede a resumption keep their order**. In `s = _a +
  _b.sent()` the machine reads `_a` after the generator resumes; `s = _a + (yield v)` would read
  it before. The value is written inline only when every read in front of it is a binding of the
  outer function that no closure captures, that the yielded operand neither reads nor writes, and
  that the outer function does not alias through `arguments`; such a binding can change only
  inside the generator. Otherwise the received value is bound first, `const _t = yield v;`, and
  the statement follows with `_t` in its place, so every read keeps the machine's order (the
  spread-around-`await` lowering of TypeScript and SWC takes this form;
  `fixtures/untranspile/hand/machine-resume-order.js`). Neither form needs an assumption.
- **Babel's regenerator is kept.** `_regenerator()` / `_regeneratorRuntime()` and
  `_asyncToGenerator` (Babel `ie 11` output, and game.js has none) are not identified: the
  runtime's body exceeds the fingerprint's binding slots, so its identity cannot be proved by
  structure, and an unknown helper is refused. Babel's async and generator fixtures keep their
  helpers unchanged.
- **game.js's generator helper is kept.** Its `__generator` is an older tslib copy that the
  obfuscator rewrote (arrow, inlined `step`), so it matches no known version and its three
  machines stay.
