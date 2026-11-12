# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.5.0] - 2026-11-12

There is no 0.4.0: its planned scope ships in this release. Most of the work is a soundness
review of the default lane, so some inputs that decoded in 0.3.1 now keep their decoder calls
(with a note) where the rewrite could have changed what the program does. `--unsafe` keeps
the rewrites webcrack makes.

### Added

- Memory and time budgets: `--max-memory` (default 4 GiB; 1 GiB in the wasm build) and
  `--timeout`, also as library options. An input that reaches a budget keeps the output so far
  or is passed through unchanged, and `--report` shows the budget and the memory peak. Only
  processing counts, not a slow reader of the output or `--fetch-wasm` (#182, #204, #236).
- `--source-map` writes a source map from the output back to the input, beside every written
  file or inline, one per unpacked module; `--input-source-map` maps through the input's own
  map. The library has `Options::source_map`, the wasm build `RunOutput.source_map`. The code
  is byte-identical with and without it (#186, #194, #261).
- One step pool per input for string-array decoding (32 times the per-decode fuel, or 2,000
  steps per input byte if larger). Once it is spent the remaining decoders stay encoded with a
  note, so a file of many spinning decoders no longer runs for hours (#225).
- Wasm glue naming: Emscripten imports of a module without a name section are named from the
  library shape and signature (#153); wasm files loaded through ES module `import`, `import()`
  or `export ... from` are listed (#160); a minified wasm-bindgen wrapper takes the name of the
  export it calls (#161); bindgen glue in a webpack 4 or 5 bundle that reaches its module
  through `import * as wasm` is named (#170).

### Changed

- Large string-array bundles deobfuscate about five times faster under `--unsafe` (a 6 MB
  Krunker build: 98 s to 20 s), with the same output (#144). Running the inline-atob RC4
  decoder natively (#156, #169), interpreter tweaks (#162) and wrapper-only rounds (#168) take
  further instructions off.
- Costs that grew quadratically or worse are gone, with the same output: patch checks per use
  and webpack 4 module ids (#203), many flattened storage objects (#179), alias flows (#212),
  fresh names, kept rotations and strictness lookups (#218), array alias chains (#221), the
  memo monitor (#222), else-if ladders, pure-call nests and method chains (#239), decoder
  machinery removal (#251), method calls on one variable (#264) and many string arrays in one
  file (#205). The alias check holds one copy of each decoded text (#253).
- Unminify drops empty statements from statement lists (#141).
- The playground's language features work on outputs up to 16 MiB, up from 12 MiB (#241).
- `docs/known-divergences/limits.md` says that regex backtracking memory is outside the heap
  budget (#272).
- CI runs the test suite in four parallel shards (#278). A release runs only from a commit on
  `main` whose CI passed, and every workflow action is pinned to a commit (#172).

### Fixed

#### Default lane soundness

- The checks for a patched built-in or a reach of the global object see more: a script without
  `"use strict"` (#187), a sloppy function's `this` (#231), a built-in reached through a value
  or code run from text (#220), the string and RegExp hooks such as `Symbol.split` (#214),
  `Reflect.set` receivers and parameter writes visible through `arguments` (#247), arrays or
  spreads as keys, object fields, `create`, `bind` and `import()` of an unknown URL (#250),
  the whole function a `Function` call builds (#210), names bound more than once (#230), a
  sloppy block function as a write to its `var` (#209, #259), program text handed to the DOM
  (#206), a `delete` through an optional chain (#219), and frame keys built at run time,
  reflective reads and code text (#270). Such programs keep their decoder calls.
- Decoder calls are not rewritten when an argument is written conditionally or twice, a sloppy
  block function shares a name, or the wrapper is async, a generator or has non-plain
  parameters; this fixes a wrong RC4 key on uglify output (#235). A call with a ternary
  argument is decoded only when every branch decodes (#276).
- Memo decoders that read another call's memo entry are refused (#143). Self-rewriting
  templates, init gates and rotator helpers are proven instead of assumed (#193).
- The string-array recognisers rewrite only shapes they can prove (#191), and six passes order
  their decisions by tree position instead of source position (#197).
- Control-flow unflattening leaves a dispatcher as written when its counter does not start at
  zero, its counter or order array is declared again, shares a name with a block function, can
  be reached by a direct `eval` or is a script global. Block-scoped declarations are not
  lifted across cases, an indirect `eval` stays indirect, a storage object declared twice is
  not inlined and statement order comes from the tree (#179, #245).
- The self-defending call controller and debug protection are removed only when the code is
  the obfuscator's own; `merge-object-assignments` folds only values that cannot throw or run
  code; `evaluate-globals` leaves `atob`, `unescape` and `decodeURI(Component)` alone when the
  program may replace them (#202).
- An `eval` layer is not peeled when its code can reach `Object.defineProperty`, `Reflect` or
  the global object (#233), and the constant folder no longer reads properties it does not
  model as `undefined` (#198).
- Code a pass copies into the program is resolved where it lands, so later passes no longer
  misread or misrename it (#256).
- When several scripts are processed together, a top-level binding another input uses or
  redeclares is not inlined, removed or renamed (#268).
- Functions are not turned into arrows or given default parameters when the program may read
  their `.arguments` or `.caller` (#232, #270).
- A stripped `function anonymous(...)` shell whose body declares names is printed inside a
  called function instead of making them globals (#163); async and generator shells are kept
  (#197).
- Control-flow object calls nested more than 256 deep are left in place with a note (#252);
  hostile parameter and domain-lock shapes no longer make the global-reach check run for
  minutes (#234); a long `!` or `typeof` chain is linear (#215).
- game.js no longer needs about 100 MB more memory than in 0.3.1 (#260).

#### Decoding coverage

- More javascript-obfuscator output decodes: plain arrays of 0.x to 2.18 that only look
  reachable through the global object (74 more version-matrix cells in the default lane)
  (#165); `arguments`-memo decoders (#147, #148, #150); arrow rotators inside declarations
  (#151); inline rotator loops (#152); an array alias reused as a temporary (#164); minified
  webpack 4 bootstraps in the safe lane (#171, #195, #199); domain locks with split strings,
  opaque tests, storage objects, chained forwarders or a reused key variable (#142, #145,
  #154, #157, #166); a for-in over the global object that only reads names (#145). Under
  `--unsafe`, the minified atob set-up (#155) and the self-defending states class (#167).
- A function holding an inline rotator loop beside other code is no longer removed as decoder
  machinery, which left the output empty (#146).
- Wrapper calls terser folded into a sequence before a concatenation (#274), a wrapper declared
  as a block function (#267) and member reads into a function-local object (#265) decode or
  merge again.
- The evaluator follows the language for loop variables after `for-in`/`for-of`, `const`
  assignment, `lastIndex`, `"@@toPrimitive"` and `toJSON`, and refuses parameter defaults that
  read their own or later parameters (#184). Replace expansions, key enumeration, `switch`
  comparisons and regexp work are charged to the budgets (#229).

#### Unpack

- Bundles are unpacked only when their runtime is proven: webpack (#185, #249), Parcel and
  Metro (#192, #200, #227, #240), esbuild (#174, #211), UMD, Bun, Turbopack and Closure
  (#223). Otherwise nothing is written and a note says why.
- browserify requires link through each module's dependency map (#190, #226); a bare
  specifier is written only where node resolves it to the target (#181); esbuild runtime
  bindings stay live (#174); webpack's `require.n` becomes `x.default` only for a proven
  harmony module, and a local `require` is left alone (#178); Parcel and Turbopack chunks link
  only where their loader loads them (#237).
- Statements that read the bundle's `this`, `new.target`, `import.meta` and the like stay out
  of `_runtime/*.js` (#201); a module that declares `require`, `module` or `exports` itself
  gets that binding renamed (#238); `--split-hoisted` uses linear memory and stops cleanly on
  a budget (#216); an arrow factory that uses the bundle's `this` is refused (#226).

#### Unminify, untranspile and rename

- Unminify no longer changes meaning in five positions (#176), merges a repeated operand into
  `??`, `?.`, `||=` or `??=` only when one read is the same (#188, #228), rewrites
  `JSON.parse` and array-callback `!!` only while the built-ins are intact (#183), keeps
  default-parameter guards that mean something else (#189), keeps merged template text valid
  (#207) and keeps spans, inlining and numeric keys right (#213).
- Untranspile identifies a helper by a SHA-256 digest and trusts it only where its binding
  provably holds it (#257). `--unsafe` class and async recovery is fixed in several shapes,
  and TypeScript's `__awaiter(void 0, …)` converts (#246). With `--unsafe`, JSX hoists keep
  evaluation order (#262).
- `--rename` keeps the bindings a direct `eval` or a `with` body can see (#177) and those that
  hoisting, function names, `.name` reads or host globals such as `status` depend on (#208),
  and no longer slows down quadratically (#266).

#### Output, CLI and wasm

- `-o DIR` never replaces an existing file or an input, refuses `$HOME` (and the Windows home
  directory), the root and the current directory, gives unsafe module ids generated names and
  reports a module file it cannot write instead of stopping (#173, #224, #244).
- Text from the input is printed with control and text-direction characters escaped in
  diagnostics, reports and banners (#255); printed code writes hidden characters in literals
  and comments as escapes (#217).
- Files without semicolons are no longer refused as nesting too deep, and chains of tagged
  templates count as nesting (#175).
- Wasm glue names found in a bundle module also apply to the combined output (#158); script
  globals and Annex B names in the glue are kept and a wasm literal is replaced by its path only
  where it is passed to `fetch` (#242); the wasm `run()` entry starts the budget clock (#244).
- Playground: hostile link fragments no longer stop the page and big results stay responsive
  (#248); budget and unpack diagnostics are labelled (#254); a fetched wasm body is capped while
  it is read, and a wasm panic gives exit code 1 (#269).

### Security

- `--fetch-wasm` is bounded (16 modules, 64 MiB, 120 s, three redirects) and contacts no
  loopback, private or reserved address unless `--fetch-wasm-local` is given; each URL is
  printed first. `--wasm-base` is joined like a browser joins it, and an internal error in the
  wasm step no longer ends the run (#180).
- `--input-source-map` cannot point the written map outside `input-map:///` through its
  `sourceRoot` (#261).
- The playground sends its Content-Security-Policy as a header with `frame-ancestors 'none'`
  and related directives, and checks a wasm URL the way the CLI does before fetching it (#258,
  #273). Release builds no longer restore the cache ordinary pushes write (#258).

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
