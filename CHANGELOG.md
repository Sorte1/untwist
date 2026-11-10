# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Memory and time budgets: `--max-memory` (default 4 GiB) and `--timeout`, also as library
  options and in the wasm build (1 GiB by default). An input that reaches a budget stops its passes
  and keeps the output so far (`budget-memory`, `budget-time`); one that goes past the memory cap
  before its passes can stop is passed through unchanged instead of exhausting the machine.
  `--report` shows the budget and the memory peak.

- Emscripten glue whose wasm module has no name section gets its imports named when the bound
  function has the shape of an Emscripten library function and the module's import has its
  signature: `_emscripten_memcpy_big`, `_emscripten_resize_heap`, `_emscripten_date_now`,
  `_emscripten_get_now` and `_emscripten_get_heap_max`. Such a module must match the glue's import
  object both ways, and a library function whose import has another signature names nothing; each
  gives a warning that says what differs.
- A wasm file loaded through an ES module `import`, `import()` or `export ... from`, as in the
  wasm-bindgen bundler target, is listed among the wasm modules to supply.
- A minified wasm-bindgen wrapper such as `function x(e,t){K.greet(e,t)}` takes the name of the
  wasm export it calls, when it calls only that export and no other function calls it.
  Calls to `__wbindgen_*` helpers do not count.
- wasm-bindgen glue in a webpack 4 or 5 bundle that reaches its wasm module through
  `import * as wasm` is named from the module: wrappers after the export they call, the
  `__wbg_set_wasm` setter and the binding it fills, when the namespace is the only value the
  setter is ever given. Every export the glue reads through the namespace must be in the module,
  or nothing is named. Webpack bundles with numeric module ids are linked too.

### Fixed

- Unminify no longer changes meaning in five shapes: a `null`/`undefined` check used as a callee,
  a template tag or a `delete` operand stays a conditional instead of becoming `a?.b`; a labelled
  loop whose head is split keeps its label on the loop (the moved statements go in front of the
  label), so `continue L` still parses; `typeof x;` stays when `x` is not declared; member reads of
  a script's top-level binding or an import are not merged into one destructuring; and an
  assignment inside `with` keeps its comma sequence. JSX reconstruction leaves an automatic-runtime
  call alone when `children` is not the last prop, appears twice or holds an array with holes.

- An unpacked browserify bundle now loads the modules the bundle loads. Each `require` whose
  request is in the module's dependency map names the file of the mapped module, so two versions
  of one package, a bundled `events` or `buffer` shim and a module reached from two directories
  link right. Every entry of a bundle with several entries runs from `index.js`, in order.
- The string-array recognisers no longer rewrite programs they cannot prove equal. A rotator loop
  moves into a function only when it is closed (no `return`, outer label, function declaration,
  `eval`, `this` or `arguments`, and its `for` head stays as written); an arrow rotator that calls
  `eval` stays an arrow; a single-use index that `arguments` can alias stays; a self-reassigning
  decoder that reads the array in a parameter default is not lifted; the cookie rotator is
  recognised only in its full template with unshadowed globals; `a["01"]` and `a["+1"]` are no
  longer read as `a[1]`; and an `eval` layer that deletes or redefines through a value that may be
  the global object is kept for `--unsafe`.
- An unpacked esbuild bundle no longer hands a module a stale copy of a binding. A binding that
  an `__esm` init assigns is now exported from `_runtime/esbuild.js` through a getter and read as
  `runtime.name`; before, a module that read it after the init ran still saw `undefined`. A bundle
  is left unpacked when `__commonJS`, `__esm` or another trusted helper is declared twice or
  assigned, when a factory holds a `with` statement (AMD, UMD and Rollup too), and a helper that
  reads `require`, `module`, `exports`, `__filename` or `__dirname` is no longer moved into the
  runtime module.

- Files written without semicolons are no longer refused as nesting too deep: the depth scan
  now ends a statement at a line break or closing brace followed by a token that starts a new
  statement, so 40,000 lines of `a = b.c(d)` or 40,000 `function f(){}` are processed. A chain
  of tagged templates (a name followed by many empty template literals) costs nesting like a
  call chain and is refused when it is too deep, instead of taking the worker to gigabytes of
  memory.
- `--rename` (every mode) keeps the names of the bindings a direct `eval` or a `with` body can
  see: the names a string-literal `eval` payload reads, every binding on the scope chain of any
  other direct `eval`, and every outer binding a `with` body reads, writes or declares. The report
  note says how many bindings were kept. Before, those bindings were renamed and the note said
  "renaming anyway". With `--unsafe`, which assumes that nothing observes scopes, they are still
  renamed, and the note says so.

- An unpacked module no longer loads the wrong file through a bare specifier. A require of a file
  under the root `node_modules/` is written bare (`require("lib/index.js")`) only when node would
  resolve it to that file; when a nearer `node_modules` directory in the output holds the same
  name, or the name is that of a node core module, the relative path is written instead. A
  sibling path starting with a dot (`.config/a.js`) is written with its `./`.
- An AMD or UMD factory that declares a function named like one of its parameters is left as it
  is. Unpacking it bound the dependency where the factory saw the function.
- `JSON.parse("…")` is no longer replaced by its literal, and `!!` is no longer removed from an
  array-method callback such as `xs.filter((v) => !!v)`, in a program that replaces `JSON.parse`
  or that array method, or passes `JSON`, `Array`, `Array.prototype` or the global object around
  as a value. The replaced function could return something else, so the rewrite changed what
  the program did. `--unsafe` keeps both rewrites.
- `-o DIR` without `--force` stops with exit code 2 when any file the run would write already
  exists in `DIR`, and names it. Before, only `deobfuscated.js` and `bundle.json` were checked, so
  a module path could replace an unrelated file. The current directory, the file system root and
  `$HOME` are refused as output directories with and without `--force`, and an input file is
  never overwritten or removed, also with `--force`.
- A module file that cannot be written (a NUL byte in the name, a path segment over 255 bytes, a
  path under another module's file, a directory that is not writable) is reported and skipped,
  and the rest of the tree is written. Before, the first such file ended the run with part of the
  tree on disk. `bundle.json` and the JSON report list only the modules that were written. A
  write that fails on disk gives exit code 1.
- A module id no longer becomes a file name that is unsafe to keep on disk: one with a control
  character, one of `: < > " | ? *` or a backslash, a segment ending in a dot or a space, a
  Windows device name (`CON`, `NUL`, `COM1`, ...), a `.git`, `.hg`, `.svn`, `.vscode`, `.idea`,
  `.husky` or `.github` directory, or a dot-leading file name such as `.npmrc`. Such a module
  gets a generated name and keeps its id in `bundle.json`. Module paths that differ only in case,
  or where one is a directory of another, get distinct names, and `deobfuscated.js`,
  `bundle.json` and `wasm/` are never used for a module.
- Any number of module ids that normalise to one path get distinct names. Before, the 1001st
  shared a name with another module, and naming them took quadratic time.
- A stripped `function anonymous(...)` shell no longer turns its body's declarations into script
  globals. When the body still declares a `var`, function, class, `let` or `const` at its own
  level, or calls `eval` directly, it is printed inside `(function () { ... })();`, so code that
  reads the global object (such as `globalThis[name]`) sees what the called shell saw. Before,
  a decoder the deobfuscator kept (or every function with `--no-deobfuscate`) became reachable
  as a global. Shell parameters the body never reads are dropped instead of hoisted as `var`.
- The safe lane proves a minified webpack 4 bootstrap like the plain one: an arrow function
  bootstrap `(e => { ... })([...])`, a module cache read as `(cache[id] || (m = cache[id] = {...},
  table[id].call(...), m)).exports`, module objects passed from a record assigned in that
  sequence, and bootstrap keys split into concatenations of strings and decoded strings. A whole bundle obfuscated with uglify-js or terser therefore no longer refuses for a
  bootstrap the check could not see.
- The safe lane keeps trusting `Object.create` and `bind` when the program defines own properties,
  under any key, on objects those natives made: `Object.create` called on the unshadowed `Object`
  and `bind` called on a function literal. Such a definition cannot be the first patch of the
  natives that made the object, so webpack's `require.d` descriptors (`{enumerable: !0, get: r}`)
  and computed keys no longer refute them. A `bind` called on any other object, and any write to
  `Object`, its definers or a built-in, still refuses as before.
- Under `--unsafe`, the self-defending states class that old javascript-obfuscator releases put
  inside rc4 and base64 decoders (`this.states = [1, 0, 0]`, `newState`, `checkState`, `runState`,
  `getState`, run once as `new o(dec).checkState()`) is dropped when its variable is read nowhere
  else and its own state test passes on the printed source, so the decoder decodes. The version
  matrix decodes 18 more cells under `--unsafe`.
- The safe lane decodes a program whose `for (const key in globalThis)` loop only reads the
  names (comparisons, `typeof`, `.length`, a string method call in a test), instead of refusing
  the unit as a global reach; the machinery is kept, since the program observes the global
  names. A key that is concatenated, stored, passed on or used as a property key still refuses.
- A domain lock whose predicate reads `s["len" + "gth"]` from split decoded strings, or whose
  arguments are folded number expressions (`0x2 * 0x2`), is recognised like the plain lock.
- A domain lock that dead-code injection has wrapped in opaque tests between two decoded strings
  (`if (dec(1) !== dec(2)) { ... } else { ... }`) in its loops and predicate is recognised: the
  branch the decoded strings pick is the one checked, the other never runs.
- The obfuscator fuzz files a domain-lock sample that also patches a built-in by a computed key
  under that sound key-reach refusal.
- A domain lock that control-flow flattening has routed through a local storage object
  (`st.aBcDe(pred, key, 4, [...])`, `st.nEqQx(s.length, n)`, `st.sAmEx(st.kQx, st.kQy)`) is
  recognised: calls into an untouched object literal run the stored function, and stored string
  fields and comparisons settle the opaque tests. A storage object that is written, reassigned or
  passed on still refuses.
- The same holds when the storage function forwards to another storage forwarder
  (`sub.vW = function (f, a, b, c) { return st.aBcDe(f, a, b, c); }`), as control-flow flattening
  emits inside a nested callback. Each step must pass every parameter on in order; one that
  reorders or drops them still refuses.
- A webpack `require.n` getter is replaced by `x.default` only when the required module is proven
  to be a harmony module, and at each use rather than once at the declaration. Before, the
  unpacked tree read `undefined` for a CommonJS module that set `__esModule` late or replaced
  `module.exports`, for a circular import, and left `g()` calls with no helper behind them on
  string module ids. An unproven getter keeps `require.n`, served by `_runtime/webpack.js`.
- A parameter or local named `require` inside a webpack module is no longer rewritten as the
  bundle's require: its arguments stay as written and it is not renamed to the runtime.
- Control-flow unflattening no longer changes what a program does in these cases; each is now
  left as written, with a note. A dispatcher whose counter does not start at zero
  (`var i = 1; while (true) { switch (order[i++]) ... }`) was laid out from its first case. A
  dispatcher whose counter or order array is declared a second time was read from the first
  declaration. A `let`, `const`, class or function declared in one case and used in another, or
  one that would become a script global, was lifted out of the switch, which removed the
  "before initialization" error the program threw or let it shadow an outer name.
- A flattened storage object that is declared twice (`var o = {...}; var o = {...};`) is no longer
  inlined from the first declaration.
- A storage object key that is not an identifier name, such as `o["a-b"] = 1`, is folded into the
  object literal as a quoted key instead of the invalid `a-b: 1`.
- An indirect `eval` stays indirect. A storage forwarder `function (f, x) { return f(x); }` called
  with `eval` was inlined to `eval(x)`, and `(true ? eval : y)(x)` was folded to `eval(x)`; both
  turned a global eval into one that reads and writes the caller's local variables.
- A statement that control-flow unflattening moved ahead of the `var` assignment it depends on is
  no longer treated as running after it. A call to a small pure function is not folded with a
  constant that is assigned later, and `var x; ... x = 1;` is not merged into `var x = 1` past a
  read that now comes first. The order of two statements is taken from where they stand in the
  tree, not from their positions in the input. A call moved behind its constant now folds.
- A script without `"use strict"` is no longer treated as strict by the built-in alias proofs.
  In such a script `this` at the top level and in a plainly called function is the global
  object, so a write through it can replace a built-in the decoder uses; a program that does
  this, or writes an unknown key through such a `this`, keeps its decoder calls instead
  of getting strings decoded with the original built-in. The call wrappers obfuscator.io emits
  for self-defending and debug protection pass such a `this` along, so some of those outputs now
  keep the decoder in the default lane (`--unsafe` decodes them).

- Program text handed to the DOM keeps the decoder calls it may reach, as `eval` of unknown text
  does: `document.write`/`writeln`, `insertAdjacentHTML`, `createContextualFragment` and
  assignments to `innerHTML`, `outerHTML` or `srcdoc`, and, when the program can create a script
  element, assignments to `text`, `textContent`, `innerText` or `src` and text nodes appended to
  it. Before, markup such as `<img onerror=...>` built from decoded strings was inlined and could
  run different code than the input.

### Changed

- Unminify drops empty statements (`;`) from statement lists, such as the one left behind once a
  dead `if (false)` block is removed. An empty statement that keeps a string out of a function's
  directive prologue stays.

- Large string-array bundles deobfuscate about five times faster under `--unsafe` (a 6 MB
  Krunker build: 98 s to 20 s) with the same output. Decoder wrappers find their function bodies
  in one walk instead of one walk per wrapper, nested control-flow forwarder calls such as
  `o.add(o.add(a, b), c)` are inlined in one pass instead of one level per pass, every
  `var o = {}; o.k = v;` run in a statement list is folded in one pass, and the rotator searches
  for several string arrays share a walk.

- A program with many flattened storage objects that are written after their declaration no
  longer walks the whole program once per object to find the branches each one decides; one
  walk serves all of them. 2000 such objects: 12.8 s to 1.2 s in a debug build.

- An RC4 string decoder whose base64 step is an inline function (javascript-obfuscator 0.x after
  uglify, as in Krunker) runs its key schedule and keystream loops natively, with the same steps
  charged and the same output: a 6 MB Krunker build spends 54 % fewer instructions under
  `--unsafe`.

- The decoder interpreter appends array elements without a resize, and the init-order check
  sorts its reference sites by value: a 6 MB Krunker build spends 2 % fewer instructions under
  `--unsafe`, with the same output.

- While the decoder wrappers of a lifted string array unwrap one layer per round, those rounds
  run only the wrapper and decode passes and skip the stages that have nothing to do: a 6 MB
  Krunker build spends 10 % fewer instructions under `--unsafe`, with the same output.

- The native RC4 decoder fills its 256-entry state array in one batch when the step budget
  covers the whole loop, with the same steps and heap charged and each element written as
  before: a 6 MB Krunker build spends 5 % fewer instructions under `--unsafe`, with the same
  output.

### Fixed

- Wasm glue names found in a bundle module now also apply to that module's function in the
  combined `deobfuscated.js`. Before, the combined file was checked against the module on its own
  and kept the minified names when another module held an object like `{ a: { z: 1 } }`, or when
  two modules each loaded a wasm module.
- Under `--unsafe`, the minified atob set-up of old decoders
  (`try { var t = Function(...)() } catch (e) { t = window }`) is dropped when the decoder later
  reuses `t` only after overwriting it, as in `return void 0 === (t = memo[k]) ? ... : t`. A
  write on one branch of a conditional or one side of `&&`/`||` does not count. A control-flow
  proxy object assigned inside a sequence (`f((x = 13, d = { ... }, g(d.k(a, b))))`) moves to its
  empty `var d;` declaration when every other use of `d` comes later in that sequence. The version
  matrix decodes 5 more cells under `--unsafe`.
- A decoder whose local alias of a function string array is later reused as a temporary
  (uglify-js output of javascript-obfuscator 4.x and 5.x: `x += t[0], t = memo[x]` or
  `e = e[0], ..., e = memo[k]`) decodes. The first write must be a top-level statement of the
  decoder, every earlier use of the alias must read one entry, and no write may assign the array
  itself.

- A function string array's rotator loop that uglify-js left inline in a function body
  (`for (var u = arr(); ;) try { ... } catch (e) { ... }` beside the module code) is wrapped back
  into its own IIFE, with the decoder wrappers used only by the loop, when nothing else reads the
  loop's variables or wrappers and the loop uses no `this`, `arguments`, labels, `return`, `yield`
  or `await`. The version matrix decodes 2 more cells in each lane.

- A function holding an inline rotator loop beside other code (javascript-obfuscator's IIFE
  wrapper after uglify-js, holding the string array, its decoder and the whole program) was taken
  for the rotator and removed as decoder machinery, so the output was empty, in both lanes.
  remove-decoder-machinery now keeps a unit whose rotator reads or writes anything other than its
  own bindings, the string array and its decoders.

- Plain string arrays of javascript-obfuscator 0.x to 2.18 (`var _0x1234 = [...]` at the top of a
  script) decode in the default lane when code such as `words[_0x5678('0x8')](' ')` only looks
  like it may reach the array by name through the global object. The check now runs again with the
  decoded keys once the unit is decoded; when it still finds a reach, or the unit does not decode,
  the input is processed again with the array left as written. The version matrix decodes 74 more
  cells in the default lane.

- uglify-js output of javascript-obfuscator 2.19 and later decodes when the rotator of a function
  string array is an arrow IIFE that uglify moved into a declaration
  (`let a = dec, b = ((() => { for (var e = dec, a = arr();;) ... })(), {})`) or to the head of a
  comma sequence: the declaration is split at the rotator and the arrow becomes a function when it
  uses no `this`, `arguments`, `super` or `new.target`. The version matrix decodes 7 more cells
  with `--unsafe` and 1 more in the default lane; 12 more cells now name the refusal that really
  blocks them instead of "array reached outside".

- The memo proof now sees `let`/`const` in blocks and loop heads and `catch` parameters as their
  own bindings (`for (let e = 0, n, t, r = 0; ...)` in a minified base64 helper no longer counts as
  a read of the decoder's parameters), and the write-only local sweep also drops nested literal
  writes such as `n = (n = -1048) - -688` and `t = t = -1013` left in webpack's
  `__webpack_require__.n` once its proxy key is decoded. The version matrix decodes 10 more cells
  with `--unsafe` and 3 more in the default lane.

- The same `arguments`-memo decoders also decode when their wrappers pass a varying second
  argument the decoder never reads: the memo proof judged every argument as read because the
  decoder captures `arguments`, and it counted a helper's own `for (var r ...)` as a read of the
  decoder's parameter `r`. The version matrix decodes 12 more cells with `--unsafe` and 3 more in
  the default lane.

- Minified javascript-obfuscator 2.x and 3.x decoders that keep their memo in the `arguments`
  object (`n = arguments` ... `n[key] = value`) decode again when a helper inside the decoder has a
  parameter or local of the same name (`function (n) { ... n.charAt(i) ... }`). The check for other
  uses of the memo matched by name, so the helper's own `n` counted as a use of the memo. The
  version matrix decodes 33 more cells with `--unsafe` and 12 more in the default lane.

- The safe lane follows a domain lock that reuses the variable holding a found key for the domain
  list in a later loop (`var x = list[i]` as the loop body's first statement): reads inside that
  loop see the list entry, not the found key, so the lock's scan of the global object is bounded
  again. A read after the loop, before the rewrite or in a closure still keeps the unit refused.
  The obfuscator option matrix now files those cells, and the debug-protection cells whose only
  failure is the global-names refusal, under the refusal that actually blocks them.

- A memoising string decoder that reads a memo entry other than its own call's key (`m[t - 1]`,
  `memo[1 + arr[0]]`) is no longer decoded, in both lanes. Its result depends on which calls ran
  before it, and untwist does not evaluate the calls in run order, so the decoded strings could
  differ from what the program prints. The memo monitor now also records missed reads, and every
  read of the memo must use the key the decoder writes.

### Security

- `--fetch-wasm` is bounded, since the URLs it requests come from the input. One run fetches at
  most 16 modules and 64 MiB within 120 seconds; a reference past a limit is named in a note and
  the run goes on. Each URL is printed to stderr, with control characters escaped, before it is
  requested. A redirect is followed at most three times and each target is checked like the
  first URL. A host that resolves to a loopback, link-local, private, shared, unspecified,
  multicast or reserved address (IPv4-mapped and other embedded forms included) is not
  contacted, the connection goes to the address that was checked, and no proxy from the
  environment is used. The new `--fetch-wasm-local` lifts the address check for input that
  loads its wasm from your own machine or network.
- `--wasm-base` is joined to a relative reference the way a browser does it, so a base without a
  path (`https://x.dev`) no longer gives `https:/pkg/a.wasm`.
- An internal error while wasm modules are supplied, fetched or named no longer ends the run
  after the code was produced: the output is written without that step, a warning names it, the
  exit code is 1 and `--json` carries `"wasm_panicked": true`.

## [0.3.1] - 2026-11-09

### Added

- The obfuscator option and version matrices file a cell under a named refusal when any later
  note names one, also behind "no string array found" in the option matrix and behind kept
  machinery, instead of calling it an unexplained miss.

- The matrices also name a decoder wrapper left in place because it is declared in a block
  (`wrapper-binding-unstable`) and a decode that bails on a const the evaluator cannot prove is
  initialised (`decoder-reads-tdz-binding`).

- Top-level calls to a small local function that updates top-level state are folded in the
  default lane when every argument is a literal: `console[test(2)](test(0), test(1), counter)`
  becomes `console.log("hello", "world", 3)`, and the function and its state are removed. The
  calls are run in evaluation order, and the fold is refused when anything else can see the
  function or the state (another function, a value use, the global object, code text).

- A javascript-obfuscator version matrix (`fixtures/obfuscator-versions`, run with
  `cargo xtask obf-options --dir fixtures/obfuscator-versions`): 723 cells over twenty releases
  from 0.10.2 to 5.9.1, string array options, top-level/IIFE/webpack 4 containers and uglify-js or
  terser after obfuscation. Its `EXPECT` records today's results, most of them misses on releases
  before 2.19.

- The version matrix files a cell whose plain string array is refused because code may reach it
  by name through the global object as that refusal (`legacy-array-reached`), not as a missed
  string array.

### Changed

- The safe lane knows which of the program's own functions a call target may hold, through
  bindings, the arguments of known call sites and returned functions. A call such as
  `memo(fn)`'s `fn(key)` no longer counts as handing `key` to outside code when every value of
  `fn` is a function of the program. A parameter of a top-level function in a classic script still
  counts as outside, since another script may call that function, unless the call passes only
  values the same outside caller supplied.
- `join` on the result of `await Promise.all(…)` or `await Promise.allSettled(…)` counts as a
  string, so printing it does not keep the decoder machinery. This holds while `Promise` is the
  built-in and no write may have the key `Promise`, `all`, `allSettled`, `then`, `constructor`
  or `join`.

### Fixed

- A run no longer depends on what ran before it on the same thread. Flags set while decoding one
  program (a decoder call with non-literal arguments seen, a constant retry requested, a pending
  write-only sweep, a lifted plain array, the per-unit alias refusals, the kept init order) are
  cleared at the start of every input, so the library and the matrix tests give the same result
  as a fresh process; before, a 4.2.2 cell decoded in the default lane only after some other
  programs had run.

- A kept cookie self-defending guard in the minified forms uglify-js and terser leave (the holder
  object carries `updateCookie` itself: block holder, `(h = {...}).updateCookie()` or
  `(o = (h = {...}).updateCookie())`) has its `removeCookie` printed on one line, so a refused
  program no longer fails its own check and runs out of memory after reformatting.

- Self-defending code inlined into a function as `(function (e, o) {...})(this, () => {...})()`
  after a hoisted `t = true` flag is removed like the declared controller form.
- A proxy object assigned before its `var` (`u = {Kmtil: function (r, n) { return r(n) }}; var u;`,
  as uglify-js leaves it in webpack modules) is declared where it is assigned, and terser's
  `let p; ... (p = {TCjgg: function (t, r) { return t + r }}).TCjgg(p.TCjgg(a, b), c)` is declared
  with its object when every other read of `p` is in that call's arguments, so the proxy calls
  inline.
- A cookie rotator in a block that rotates the array itself (no alias), and whose block `var` is
  shared with a later `var` of the same function, is wrapped back into a rotator when the later
  code overwrites that `var` before reading it.

- A string array inside a function decodes when uglify-js left its checksum loop behind the
  hoisted decoder and its aliases (`let t = [...], n = r; function r(e) {...} for (var x = t, e = r;;)
  try {...}`): the loop is wrapped back into a rotator and the declarations around it stay where
  they are. An array alias that the rotator only uses for `push` and `shift` is resolved, and an
  alias is resolved when the array's name means the array at every read of the alias, instead of
  only when the name is unique in the program (a block function sharing the name still refuses).

- javascript-obfuscator 5.x decoders minified by terser decode: the memo proof now accepts the
  index shift written as `t -= 0`, and a base64 helper's own locals no longer count as reads of the
  decoder's second parameter when they share its name.

- A decoder argument whose local is assigned from never-written literal bindings (terser's
  `o = -E, W = U, dec(o - -331, W)` with `const E = 362` and `var U = ...` at the top of the
  bundle) decodes: those bindings are inlined first, under the same initialisation-order proof as
  a constant read directly by a decoder call. uglify-js's `n = (n = -1048) - -688` is read as the
  literal arithmetic it is.

- A decoder argument read from a local is no longer replaced by an earlier literal assignment when
  the local is written again by its own declaration's initialiser, a later `var` with an
  initialiser, a destructuring declaration, a `for (var ... in/of)` head or a sloppy-mode block
  function: `var t = a[(t = -937, d(t))](0); f(t)` replaced the later `t` with `-937` in both lanes.
  Chained literal writes such as `e = t = -937` now count as literal assignments of both locals.

- The dead-code pass folds a string-literal comparison on the left of `&&` or `||`
  (`"mVBnO" === "mVBnO" && !f.o(x, e) && ...`, as uglify-js writes injected dead code), as it
  already did for `if` and `?:`, and drops one that cannot change the result from the middle or
  the end of a chain (`a && "SedNn" === "SedNn" && b`, `if (a && "x" === "x")`). A member read
  it would hand to a call is still kept.

- A decoder wrapper that reads a literal `const` of its own function (terser keeps
  `const C = "0x396"; function g(n, x, e) { return dec(e - -C, n) }` beside the rotator) gets the
  literal, so the rotator and the calls through the wrapper decode. Only reads of a const that is
  never written are replaced, and only when every use of the wrapper comes after the const's
  declaration in the same function.

- The default lane no longer decodes when the program may replace a built-in the unit's decoders
  or rotator use through a variable, parameter, `this` or computed object that can hold it
  (`var P = Array.prototype; P.shift = f`, `Object.defineProperty(P, "shift", ...)`, an aliased
  `Array`, `[].__proto__`). Before, such programs printed wrong strings. The refusal names the
  write's line and column; `--unsafe` assumes the built-ins are untouched and still decodes.
  Bundles that pass prototypes through unknown calls, such as core-js polyfills next to
  javascript-obfuscator's self-defending code, are now refused by default and need `--unsafe`. A
  definer call whose keys are all literal and none of them used does not refuse. A function an IIFE returns
  binds its parameters at the calls of the variable it is stored in, a call whose decoder argument
  decodes is a string, and `fn.apply(...)` / `fn.call(...)` on a parameter that only holds program
  functions is not a definer call unless the program stores a definer under such a key.

- The dead-code pass now also folds a constant-test conditional whose chosen branch is a member
  read (`"ab" === "cd" ? module.x : d[x]`), as dead-code injection leaves them inside functions;
  it still keeps the conditional when it is called or used as a template tag, where folding would
  change `this`.

- After decoding, assignments of literals to a local that nothing reads any more are dropped from
  the comma sequences that held them (uglify-js and terser leave `obj[e = 353, t = 356, dec(...)]`
  when they inline a decoder wrapper), together with the local's empty declaration, so the proxy
  object around them can be inlined. A `let` assigned before its declaration runs is left alone.

- Decoder wrappers that pass the decoder a single argument (terser drops the unused second one:
  `function l(n, t, e, o, r) { return i(t - -494) }`) are inlined like two-argument ones, and a
  wrapper called where it is written (`function (n, t) { return i(t - 1) }(0, 2)`) is inlined
  whenever decoding saw a decoder call with non-literal arguments, not only after a plain array
  was lifted.

- A plain string array inside a function or webpack module whose decoder is a function
  declaration is decoded when nothing runs before the array in that function body; only code
  in the same body can call the hoisted decoder before the array exists.

- A string array rotator inside a function is no longer run with the standard `push`, `shift`
  or `parseInt` when the program replaces one of them: code before the function call may have
  done so, so the default lane now judges the rotator's built-ins like the decoders' ones.

- The default lane also rotates a plain string array inside a function or webpack module ahead
  of time when nothing runs before it in that function body and no write in the program can
  replace a built-in its rotator uses (`push`, `shift`, and for the cookie rotator its regular
  expression and `toString` calls); otherwise the array is left undecoded with a note.

- Old javascript-obfuscator rotators whose count or checksum target is written as literal
  arithmetic (the `numbersToExpressions` option, e.g. `0xdd7*-0x1+-0x542*-0x2+0x1`) are recognised.

- The readable self-defending cookie rotator of javascript-obfuscator 0.16 to 2.9 (a named guard
  function, `if (!check) ... else if (check) ...`, and a count passed through the rotator's
  parameter) is recognised like the compact one: the rotation is applied only when the guard's
  format check passes on the text of `removeCookie`.

- Inside a function, terser folds the old counting rotator into its caller as
  `e = arr, n = 107, function (n) { for (; --n;) e.push(e.shift()); }(++n)`. That form is
  recognised when the two variables are declared with `var` in the same body and used nowhere
  else, and their now unused declarations are removed. A bare `var x;` before an array no longer
  counts as code running before it.

- uglify-js turns the same rotator into a loop, `for (var x = arr, n = 107; --n;) x.push(x.shift())`,
  sometimes declaring the array in the loop head. That loop is recognised under the same
  conditions, and a few bare `var` statements between the array and an assigned rotator are
  skipped.

- A function declaration that decodes a plain array is also found when it comes before the array,
  as uglify-js hoists it; it is used only when nothing runs before the array in that body.

- uglify-js also inlines the checksum rotator of javascript-obfuscator 2.10 and later into a bare
  `for (;;) { try { ... } catch (e) { ... } }` loop after aliases of the array and the decoder.
  The loop and the aliases only it uses are wrapped back into the rotator function. A checksum
  rotator whose parameter reuses the array's name is accepted too.

- uglify-js folds a plain string array into the head of the next `for (var ...)` loop
  (`for (var e = ["..."], x = [o("0x1")], l = 0; ...)`). When a function beside the loop reads
  the array, the leading array declarations are moved in front of the loop, so the array is found.

- terser inlines a decoder that is called once into its call site, either as a function called in
  place (`console[function (n) { return f[n -= 0]; }("0x0")]`) or as a sequence through a variable
  (`var c; console[(c = "0x0", f[c -= 0])]`). Inside a function, such a site is turned back into a
  decoder declared beside the array; the sequence form only when the variable is a `var` of that
  function used nowhere else.

- uglify-js and terser also inline the cookie rotator of old javascript-obfuscator releases into a
  function body: the array alias `n = arr`, the countdown function (hoisted, or assigned as
  `i = function (e) {...}`) and the `(h = {...}).updateCookie() ? ...` choice, terser's
  `(o = (h = {...}).updateCookie()) ? o ? ... : ... : ...` form, or uglify's block holder. When
  only code that does not run sits between the array and the alias, and the alias, the countdown
  function and the holder are used nowhere else, they are wrapped back into the rotator function
  right after the array. terser's reader that keeps the cookie lookup in a new variable is read
  too.

- A decoder wrapper with a single parameter (`function w(x) { return b(x); }`) is inlined like
  other wrappers.

- Output that keeps javascript-obfuscator's self-defending code (for example Krunker 3.2.x in the
  default lane, where the string array stays undecoded) no longer hangs when it runs. The cookie
  rotator's `removeCookie` function, a call controller built and called in one expression or inside
  a comma sequence, and a guarded function that opens with a wrapper variable are printed on one
  line, so the checks that read their source pass as they did on the input.

- String arrays, decoders and wrappers declared with `let` or `const` (javascript-obfuscator 0.22
  and later reuse the program's declaration kind) are found like `var` ones; a lexical array keeps
  a lexical binding when it is lifted. Machinery bound by `var`, `let` or `const` that may be called
  before its declaration runs, or a `const` that code assigns to, is left undecoded, since both
  throw.

- Plain string arrays rotated by the parseInt checksum rotator (javascript-obfuscator 2.10 to
  2.18) are decoded: the array is lifted into the array-function form the later releases use, a
  self-reassigning decoder reads it through a local alias as they do, and the rotator itself is
  still proven and run. A decoder or array bound by `var`, `let` or `const` to a function
  expression before the rotator now counts as set when the rotator runs. A function decoder of a
  plain array is left alone when code runs before the array's declaration.

- Plain string arrays whose machinery was minified by uglify-js or terser after obfuscation were
  not found: rotators written as arrow functions, the counting loop inlined into the rotator
  (`for (var r = N; --r;)`), the checksum inlined into a one-parameter rotator, the rotator
  joined to the decoder's declaration with a comma, and decoders of the form
  `return (d = function (...) {...})(a, b)` or `return a = +a, arr[a]`. These are now put back
  into the form the unminified builds use before the array is lifted. A refused plain array now
  keeps its reason note when other passes changed the program in the same run.

- A decoder call whose argument is a function parameter set to a literal just before the call
  (`(e = 1197, dec(363, e))`, as uglify-js leaves inlined wrappers) is decoded like one on a
  local variable; a parameter the function's `arguments` object can alias is left alone.

- Self-defending call controllers minified by uglify-js or terser are removed like the unminified
  ones: an arrow controller called where it is built (`(() => {...})()(this, cb)`), a controller
  inlined into its caller with its flag set beside it (`o = !0; var o, r = (n, t) => {...}`), and a
  guarded function that declares its result before assigning it.

- Self-defending cookie rotators minified by uglify-js (the counting function declared inside the
  rotator, the holder assigned in the test or built in a block, the count set through an alias)
  or by terser (an unused literal declaration, a `var` after the reader's return) are read like
  the unminified ones: the rotation is applied only when the removal text passes the format check.

- `--unsafe` also drops the lazy `atob` set-up of old base64 and rc4 decoders when a minifier
  inlined it into the decoder (`try { var n = Function(...)() } catch (t) { n = window }
  n.atob || (...)`, or `(r = (() => {...})()).atob || (...)`), when the variable that holds the
  global object is overwritten before anything reads it again.

- String arrays from old javascript-obfuscator builds (a plain `var a = [...]` with a counting
  rotator and a `var b = function (t, x) { ... a[t -= 0] ... }` decoder) were not found, and the
  run said `no string array found`. They are now decoded when the array is only read by its
  decoder; the default lane also needs the usual global-reach and patched-built-in checks to pass.
- `--unsafe` no longer refuses a top-level string array or decoder just because some string in
  the program spells its name; it refuses only when the name is reached through the global object
  or literal code text. Old builds with one-letter machinery names (`a`, `b`) now decode there,
  including the lazy `atob` set-up their base64 decoders install.
- Old-builder string arrays inside a webpack module or function, with a self-defending cookie
  rotator, rc4 decoders and wrappers in `for` heads or called in place, were not found. They
  now decode under `--unsafe`; the default lane decodes them when no rotation has to be applied.

## [0.3.0] - 2026-11-07

### Added

- `--wasm-check off|on|auto` (and Wasm check in the playground settings) decides whether the
  output is scanned for embedded wasm modules. The default is now `off`, which only notes when the
  output may hold one; `auto` extracts modules that have imports or exports, so feature-detect
  probes passed to `WebAssembly.validate` are left alone; `on` extracts every module it finds.
  Supplied and fetched modules work in every mode.
- Emscripten glue is named when its wasm module has no name section: an export wrapper
  (`_malloc = Module._malloc = exports.f`) takes the public key's name when the export is a
  function, the binding of the memory and table exports becomes `wasmMemory` and `wasmTable`,
  `Module.asm` counts as the exports holder, and the object handed to the instance as imports
  becomes `wasmImports`. Import functions whose minified names the module does not confirm stay
  unnamed.
- The names wasm glue gets in a webpack bundle's unpacked modules are also applied to the
  combined `deobfuscated.js`, each inside its own module function. If that copy does not hold the
  same bindings, a warning says so and it keeps the original names.
- wasm glue in a webpack bundle is named across modules: the functions an async or sync wasm
  module imports are renamed in the modules that export them, and the bindings that read its
  exports in the modules that import it, after the same check that every key and read exists in
  the supplied module. Webpack 5 bundles using `syncWebAssembly` now unpack.
- Unpack writes wasm modules embedded as base64, data URIs or byte arrays (wasm-bindgen,
  Emscripten `SINGLE_FILE`, webpack inline assets) to `wasm/*.wasm` beside the output and
  points the literal at that file. Each payload is checked for the `\0asm` header and a size
  limit, decoded statically and never run; its import and export names name the JS glue when
  they match it. Remote wasm (`fetch`, `new URL`, `locateFile`, webpack's hashed
  `.module.wasm`) is listed in the report with the name to supply it under; nothing is fetched.
- `--wasm-dir DIR` supplies wasm modules the input loads from elsewhere, matched by file name
  (hashed names included), and `--wasm NAME=PATH` overrides one. `--fetch-wasm` downloads the
  rest from absolute URLs, or relative ones against `--wasm-base URL`; without it nothing is
  fetched. A supplied or fetched module that does not cover the glue's imports and exports is
  reported and names nothing.
- The playground lists extracted wasm modules with a download button each. Dropping `.wasm`
  files on the page matches them to the modules the code loads by name. Each remote module can
  be fetched after a prompt naming its URL (relative ones ask for a base first); an "Always
  fetch wasm" setting, off by default and kept in the browser only, skips the prompt.

### Changed

- `dead-code` now also removes branches whose condition is a literal constant: `if (false)`, `if (1)`, `while (null)`, and the `?:`, `&&` and `||` forms of the same. It keeps `var` declarations of the removed code (without initialisers), leaves a dead branch with a function declaration in sloppy mode, and never touches a labelled statement or folds a member reference out from under a call, `typeof` or `delete`.
- Terser-minified stateful decoders (base64 and rc4) decode in the safe lane: `void 0 === D.k` (either
  side, strict or loose) counts as the init gate's undefined test when `void` takes a literal, and the
  reset and temporal dead zone checks resolve the decoder's locals by scope, so a helper reusing short
  names such as `n`, `f` or `a` no longer refuses the unit.
- Rounds that add or remove scopes and bindings patch the scope tables instead of rebuilding
  them, and references left on a removed binding resolve to the outer one: game.js runs about
  3 % fewer instructions with the same output.
- The default lane keeps the decoder machinery less often beside code that may hold the global
  object: calls to `JSON.parse`, `Math`, `Number`, `parseInt`, `Promise.all` and other intrinsics
  that neither print nor enumerate, and calls to `require` or to a required `path` or `url`, no
  longer count as observing its names. The last rests on a stated premise, see the README.
- The evaluator runs the obfuscator.io base64 and rc4 string-decoder loops natively, with the same step
  accounting and the same output; decoding costs less memory and time on inputs that use them.
- Unminify rounds after the first skip unchanged statements: about 6 % faster on large inputs.
- Unminify finishes the body an `if` rewrite builds in the same round, and `!` inverts nested
  comparisons at once: game.js needs 3 unminify rounds instead of 9, about 8 % fewer instructions.
- The playground worker keeps the decoded strings in one buffer while it runs, so the 10 MB sample peaks about 26 MiB lower (190 MiB instead of 216 MiB).
- The default lane's decode step budget grows with the distinct decoder calls a unit decodes
  (50 M plus 500 k a call, at most the 1 B `--unsafe` uses), so the high preset's rc4 strings
  no longer run out of steps.
- A decode unit whose string array, decoders, rotator and call arguments are unchanged since
  an earlier round reuses that round's decoded values instead of running again, so a high-preset
  unit that is decoded and then kept as written no longer decodes twice: about 30 % fewer decode
  steps on the high preset.
- The native rc4 decoder loops also run when unminify's prepare passes have already split the
  decoder's comma sequences into statements, which is how the pipeline sees them: the high
  preset needs about 84 % fewer instructions (7.0 s to 1.2 s for its ten samples), with the
  same output.
- oxc 0.157: output is unchanged; parsing and printing take about 12 % fewer instructions, and
  game.js about 3 % fewer overall.

### Fixed

- The default lane no longer replaces a decoder argument that reads a top-level script `var`,
  function or `let` by its literal when code that may hold the global object (`globalThis.k`,
  `window.k`, top-level `this.k`, `Reflect.set`, `Object.assign`, computed keys) or code from text
  may write that name; `var k; k = 1; globalThis.k = 2; d(k)` decoded with `1`. When such code may
  only observe the name, the declaration is kept. Modules and function-local bindings still decode.
- The default lane decodes a global string array whose function or decoder name is also spelled
  by a string in the program, such as webpack 4's `'a'` getter key under mangled names; whether
  code may reach the machinery by that name is now judged where the global object is reached,
  instead of refusing on any spelling. A spelling under a direct eval still refuses.
- The default lane no longer refuses an obfuscated webpack 4 or webpack 5 bundle because of its
  bootstrap: a module's `exports` and `module` are proven to be objects the bootstrap created, and
  an AMD factory that ignores `require` no longer leaves the bootstrap unproven. A setter on the
  module cache or a read of `require.c` still refuses.
- A header-only WebAssembly feature-detect (`new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])`)
  is no longer taken for an embedded module, and output holding a real one is parsed once, not
  twice, to extract it and name its glue (game.js about 5 % faster).
- The default lane no longer inlines a variable passed to a decoder from an assignment that a write
  in the same statement has replaced: `f(d(k), [k = 2][0], d(k))` printed the wrong string. A literal
  assigned earlier in the same sequence, call or block (`(k = 2, d(k))`, `w(1, (k = 5) - 2, d(k))`,
  as terser prints them) is now inlined where no other write can run between.
- A decoder alias declared after statements that run no code (`void 0;`, `{}`, `;`) is no longer
  refused as possibly read before its declaration; such statements cannot call the hoisted
  functions that read it.
- javascript-obfuscator output passed through terser decodes: a decoder that indexes the array call
  directly and a string array function that rewrites itself inside its return are now recognised.
  Such samples kept their string array.
- A decoder wrapper call whose arguments assign locals, as terser emits after a high preset
  (`w(1, (t = 5) - 2, t)`), is inlined with the assignments kept in order, in both lanes.
- The default lane decodes an ES module whose imports load only `node:path` or `node:url`; such
  an import runs no program code and touches no built-in, so where it sits beside the rotator no
  longer matters. Any other import still keeps the string array.
- An object literal before the rotator whose values are literals or hoisted functions
  (`var o = { k: f }`) is read as running no code, as the debug protection's holder is.
- Empty or inert blocks and expression statements that run no code (`{}`, `void 0;`) before
  the rotator no longer stop a string array from decoding.
- A string array whose array function, decoders and rotator all sit in one function body,
  such as a whole program wrapped in an IIFE, now decodes in both lanes.
- A rotator that calls the string array function by name once, as its first statement, and
  a rotator loop whose body is a bare `try` (both as terser prints them) are now trusted.
- Decoder aliases read inside the rotator, held in a one-property object, returned by an
  empty IIFE or wrapped in `(0, alias)` are inlined in both lanes, so their string arrays
  decode instead of being kept whole.
- The default lane decodes beside a getter or method of an object literal that only its own calls reach, which
  was taken for a hold on the global object through `this`.
- The default lane decodes javascript-obfuscator's domain lock: its scan of the global object's
  keys is proven unable to pick a name of the decoder machinery.
- The default lane reads the keys of an object built by keyed assignments and then aliased
  (javascript-obfuscator's transformObjectKeys) in a rest or spread, as it does for an object literal;
  such a rest was refused as one that may read any key. Its getters and methods, including the
  functions assigned to it, count as bound to it like an object literal's.
- The default lane decodes beside a `set` call on a local instance of a plain class (`v.set(x, y, z)` on
  `const v = new Vector3()`), which was taken for `Reflect.set` and refused, and beside a key write on
  `Object.assign({}, x)`, which was not taken for a fresh object.
- The default lane removes the decoder machinery beside `set`, `get`, `has`, `delete` and `add` calls on a
  fresh `Map`, `WeakMap`, `Set` or `WeakSet`, which were taken to observe the names of the global object
  passed to them.
- The default lane decodes beside a fresh `new Map()` or `new WeakMap()` that is read and written only
  through its own `get` and `set`; those calls were taken for `Reflect.get` and `Reflect.set` and refused.
- A decoder wrapper whose binding is written again, or a `var` wrapper called before its
  declaration runs, is no longer inlined; both printed the wrong value in the default lane.
- An alias of a decoder wrapper (`var k = wrapper`), and wrapper calls inside the rotator,
  are resolved, so string arrays reached only through such wrappers decode in both lanes.
- The default lane now removes base64- and RC4-encoded string arrays and their decoders where it
  already removed plain ones. A decoder's own cache no longer counts as code that may hold the
  global object.
- The default lane no longer refuses a decode because of an object rest or spread, or a Symbol-valued
  computed key, on an object created in the same unit; only what such a key can read is judged.
- The default lane decodes a string array when a key or code text is built from decoded strings, such as
  `String[d(1)](...)` or `Function(d(1) + d(2))`; these were refused as a possible patch of a built-in.
- A self-defending check is removed when a cleanup left bare `var x;` declarations inside its call
  controller. It survived, and once the output was reformatted its `(((.+)+)+)+$` test hung.
- The default lane no longer overflows its stack on a variable initialised from a read of
  itself (`var a = b[a]`) or a sealed method that calls itself in an `if` test; such inputs,
  seen in dead-code-injected obfuscator output, used to stop the worker and pass through
  unchanged.
- The default lane reads `cache[d(1)](key)` on a fresh `new Map()` by the method name `d(1)` decodes to, so a
  memoize cache whose method names were moved into the string array no longer refuses the unit.
- The default lane reads a control-flow storage object's members as the text they build, so the global-object
  getter `Function(s.Ncuch(s.IvYXD, d(1)))` and the forwarded `s.PwbRa(Function, text)` that control-flow
  flattening leaves are judged by their text instead of refusing a stateful unit.

## [0.2.1] - 2026-11-05

### Fixed

- `--unsafe` decodes a string array beside code that may hold the global object, such as
  core-js's global lookup or Babel's spread and construct helpers, as webcrack does. Such
  bundles were left fully encoded; the default lane still refuses them.

## [0.2.0] - 2026-11-04

### Added

- `--unsafe` now decodes everything webcrack 2.16 decodes on every free javascript-obfuscator
  option, checked by an option matrix of 1,100 generated samples in CI.
- Calls to pure lookup helpers with literal arguments fold to their value, so
  `console[dec(2)](dec(0), dec(1))` prints as `console.log("hello", "worold")`.
- A whole-program value-flow analysis lets the safe lane decode string arrays whose reads
  escape into code it can prove inert.
- Decoders that reset their memo cache (newer javascript-obfuscator releases) are decoded.
- Playground: refusal explanations, pass-by-pass stepping, a decoded-strings table and linked
  selection.

### Changed

- `--unsafe` assumes built-ins are pristine even when the program patches one, as webcrack does,
  and gets a larger step budget.

### Fixed

- Regex escapes such as `\b` no longer count as spelling a decoder name.
- Decodes deferred while wrappers were still being inlined are retried instead of silently
  skipped.
- Computed `Object.keys` keys and unused hoisted variables left behind by dead-code injection.
- The playground's Steps view works in Safari under the strict content security policy.

## [0.1.0] - 2026-11-01

First public release.

### Added

- Deobfuscation of javascript-obfuscator / obfuscator.io output: string arrays with their
  rotators, base64 and RC4 decoders and wrappers, control-flow proxy objects, `switch`
  flattening, self-defending and debug-protection code, and constant folding. Decoders run in a
  sandboxed, step-budgeted interpreter; the input is never executed.
- A safe lane (the default) that decodes only what it can prove from the program text, and an
  unsafe lane (`--unsafe`) that assumes built-ins are not patched and adds heuristic passes.
- Unminify and untranspile passes, including TypeScript, Babel, SWC and esbuild helpers.
- Unpacking of webpack 4/5, browserify, esbuild and Bun, SystemJS, AMD, UMD, Parcel, Metro,
  Turbopack and Closure bundles, with opt-in splitting of Rollup and Vite scope-hoisted bundles
  and unpacking of several input files together.
- Identifier renaming (`mangle`, `simple`, `unique`) and JSX reconstruction.
- The `untwist` command-line tool, the `untwist` library crate, and the `untwist`
  WebAssembly package used by the playground at https://untwist.dev.
