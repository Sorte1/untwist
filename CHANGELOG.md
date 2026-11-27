# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Collection generator fixes (review r18b B3, B4, B5, B6, B7, B8, B9): `pack` gives a package's rows back to it when another package vendors most of them (eventemitter3 inside phaser, punycode inside markdown-it) instead of dropping shapes 3+ labels share for everyone, and drops rows named like bundler runtime code (`__webpack_require__`, `__toESM`, ...); string keys are generated again (an empty build folder filtered every literal out) and `pack` refuses a collection without any; a nested function name credits its function only within the package of its enclosing module and no longer makes a wrapper row; a source-map source that is not an installed script (`src/utils/common.ts`) is no longer taken as a module name; `select.mjs` reads a one-package download answer and does not record a name as missing when the request failed, and its usage says that `--wanted` names go to npm. The shape-epoch canary also digests pinned generator builds (`fixtures/library-collection/canary`: esbuild, webpack 5 production, vite 8 and rollup + terser builds with classes, optional chaining and generators), and new canary inputs can be recorded without an epoch bump while recorded rows still cannot move.
- `cargo xtask library-collection build` also records file-order rows (table role `o`) from unminified rollup and from vite builds: for every library file with 16 or more top-level statements in the hoisted output (the body of the bundle's IIFE when there is one), the rename-robust shape of each top-level statement in order (`untwist_rename::library::shape::statement_shapes`), its position and the build family, from the default lane (the `--unsafe` lane only for a file the default lane does not give). They are kept apart from the match rows (`Table::order`), stay out of the bloom filters and the pack filters, and the matcher ignores them; they are the input for finding library boundaries inside one hoisted file. Function shapes are unchanged, so the shape epoch does not change. Shards grow by 0 to 28% (three 1.94 MB to 2.25 MB, react-dom 324 KB to 377 KB, lodash-es unchanged).
- `--library-imports` (opt-in, needs `-o`) replaces an unpacked CommonJS module by `module.exports = require("pkg/file")` when its whole-module shape equals a complete-module row (new table role `c`) and writes `package.json` with the pinned versions. A module is kept (and listed with the reason) when it requires a module that is not replaced too, requires a computed name, reads bundler state (`__webpack_require__`, `require.main`, `module.parent`), is an ES module, or when no single version fits every replaced module of its package; any `require.cache` or `module.hot` in the bundle turns it off. The JSON report gains `library_imports {replaced, kept, pins}`. It is part of the CLI and of the Rust library (cargo feature `library-imports`, on by default); the wasm build leaves it out. `cargo xtask library-collection build` marks a module row complete when webpack (named build, no concatenation) used every export of the file, the package lets the file be required by that specifier (`exports` map or none), and every lane that unpacked it gave the same shape; the specifier is kept in the row. The matcher treats complete rows like module rows, so the shape epoch does not change. The generator also records module rows of the unpacked files below the output's top folder, which it skipped before, so a collection now has whole-module rows (review r18b B2).
- `untwist libraries update [--url URL] | status | clear` keeps a library collection in a cache directory (`$XDG_CACHE_HOME/untwist/libraries`, `UNTWIST_LIBRARY_CACHE` overrides it): `update` fetches `collection.json` and the shards and filter file the cache lacks, checks each against its size and SHA-256, writes them atomically and removes the files the new index no longer names. `--libraries` gains `cache`, now the default (the built-in tables plus the cached collection, when one exists), and an `http(s)` URL, from which only the shards the program's blooms want are downloaded and kept in the cache. Nothing else uses the network. Fetches reuse the `--fetch-wasm` client (ureq with rustls): at most three redirects, size and time limits, and no proxy or private addresses unless the collection URL itself is on the loopback host. `--modern` without core-js now says `run untwist libraries update`.
- Collection loader fixes (review r18b B1, B10, B11, B26): a generated package is fetched only when at least 3 of the program's shape keys hit its bloom filter and one of them is a unit of 60+ tokens, or the whole program's key hits it (curated packages still on one hit); filters use 16 bits and 11 hashes per key. A 2 KB input loads 5 of 476 shards instead of 81, a 134 KB lodash build 100 instead of 451 (0.19 s and 83 MB against 0.81 s and 259 MB); whole apps still load most shards, since short shapes repeat across packages; the output is unchanged. A shard or filter file is read only when it is a regular file of the size the index gives (a FIFO or `/dev/zero` no longer hangs or fills memory); `--libraries` with a path that is neither a folder holding `collection.json`, a `collection.json` nor a `.table.gz` is an error naming it; an index that is not UTF-8 is treated like one that is not JSON. Loaded shards keep their unfiltered rows and are filtered against the built-in tables on every rebuild, so the result no longer depends on whether a table or a shard was loaded first.
- `cargo xtask library-collection pack` splits a package whose shard would pass 256 KB into one shard per major version (the first non-zero version part, so three 0.150 and 0.180 are two majors); a row goes to the shard of its newest major. Each shard gets its own bloom filter (`id#index` in the filter file), and a collection loads only the shards whose bloom the program hits; `library_wanted` in the wasm API now answers shard SHA-256s instead of package ids.
- `--libraries <builtin|none|DIR|FILE.table.gz>` (repeatable) reads an external library collection: `collection.json` (schema `untwist.library-collection/1`, shape epoch), one SHA-256-named shard per package and one bloom file. Only the shards whose bloom filter the program's shapes hit are read, each checked against its size and SHA-256; a collection for another shape epoch is refused with a warning and the built-in tables are used. `--report json` gains `collection {version, epoch, packages, shards_loaded}`. The wasm API gains `load_library_collection`, `library_wanted` and `load_library_shard`. Nothing is fetched over the network.
- Collection rows are held to a stricter rule than the built-in tables: a package from a collection is named in a file only with at least 3 distinct hits there (or one whole-module hit), a shape that 3 or more collection packages share is not evidence for any of them, and collection rows whose shape the built-in tables already hold (vendored babel / tslib helpers) are dropped when a shard loads. The built-in tables match as before.
- `cargo xtask library-collection select|build|pack|hints` with `tools/library-collection` builds an external library collection: packages ranked by npm downloads (plus a locally kept list of names `hints` finds in inputs, no telemetry), versions up to 80% of last-week downloads plus the newest of each major, entries from `package.json` `exports`, every bundled file credited to the package that owns its `node_modules` path, builds with rollup 4 + commonjs, esbuild, webpack 5 (named and production with module concatenation), vite 8 (rolldown + oxc) and terser / esbuild minify, pinned tool versions and deterministic output. `pack` drops shapes the built-in helper tables hold, helper-named rows, bundler runtime shapes and shapes 3 or more packages share, and writes one SHA-256-named shard per package plus one bloom file. The collection itself is not in the repository.
- js-confuser 2.x support (`fixtures/js-confuser`, `cargo xtask obf-options --dir fixtures/js-confuser`).
  String concealing decodes: each `STR(start, length)` getter call whose decoder, getter and the
  `getGlobal`/`bufferToString` prelude match js-confuser's templates (up to names, layout and its
  own minify pass) is replaced by the string, read as UTF-8 the way `TextDecoder` reads it; a
  string that is not well-formed UTF-8 or starts with a byte order mark is left as a call.
  Global concealing calls of standard globals (`console`, `Math`, ...) become the global, the
  calculator's `calc("key", 3, 4)` folds, the duplicate-literal array's constant reads become the
  literals, `"key" in dummyFunction` opaque predicates fold to `false`, and `lock.selfDefending`'s
  newline checks go when no checked function's source holds a newline. `lock.integrity` checks go when the checked function's source text, hashed with js-confuser's cyrb53 the way the check hashes it, gives the expected value; before, a reformatted output could hang on them. In the default lane each
  needs the built-ins it reads unpatched, and the machinery is removed only when no code may
  observe the global object's names. Nothing is folded or removed when another input reads,
  calls or writes a machinery name or a checked function, when a line break follows `return`
  where the template has none, when the integrity cache key is a property functions already
  have (`prototype`, `bind`, ...), or when a calculator repeats a parameter name. In the default
  lane the built-ins scan for opaque predicates covers their dead branches too, and a checked
  function reached through the global object keeps its integrity check.
- js-confuser `dispatcher` and `flatten` are undone. A dispatcher's function table becomes function
  declarations again and its `(payload = [args], dispatcher("key"))` calls become plain calls; a
  function the dispatcher hands out as a value keeps the name `fn` and `length` 0 it had there
  (`function fn(...[a, b])`). A flattened function's body goes back into its
  wrapper, its scope object's getters, setters, `typeof` getters and call proxies become the
  variables again, and the parameters stay `...[a, b]` so `length` stays 0. Both are matched on the
  AST; they refuse a body that reads `this`, `arguments`, `new.target` or a dispatcher local, a
  payload or scope object used any other way, a name that would bind to another variable after the
  move, machinery another input names, and in the default lane a patched `Object`, `Array`,
  `Function` or `Symbol` the machinery relies on.
- js-confuser `customStringEncodings`: a string decoder that is not the base91 template is decoded
  by running it in the sandboxed evaluator on each getter call's slice of the packed string, when it
  takes one plain parameter and reads no name declared outside itself. In the default lane the
  built-ins it can reach through its values (`String`, `Array`, `Number`, `Boolean`, `Function`,
  `Symbol`, `Object`, `Math`, `JSON`) must be unpatched, as for the base91 decoder.
- js-confuser string concealing with the `dispatcher` on: when a dispatcher's keys are concealed
  strings, the string decoder is judged on the program with those strings in place and the
  dispatcher undone (under the dispatcher's own checks), so its `fns[name].apply(this)` and
  `fnLengths[name]` reads no longer keep the strings; the dispatcher pass then restores the
  functions from the decoded keys.
- CommonJS input: a `.cjs` file (or any input with `--source-type commonjs`, also a flag of the
  wasm API) runs inside node's module wrapper, so its top-level `var`, `function`, `let`, `const`
  and `class` names are local to it: not global-object properties and not shared with other
  inputs. Top-level `this` is not the global object there. An input whose own level reads
  `arguments`, `new.target` or `eval` keeps today's script-or-CommonJS treatment.
  `--source-type script|module|commonjs` overrides the extension.
- `unread-decoys` removes, in CommonJS input only, top-level functions and literal data
  declarations (literals, arrays and objects of literals, `Object.freeze` of them when it is
  unpatched) that nothing outside the removed set reads, in a program without direct `eval` or
  `with`; a `void f;` statement goes with `f`.
- With `--rename smart`, a binding exported as `module.exports = { key: f }`, `exports.key = f` or
  `module.exports.key = f` takes the key as its name.
- `dead-code` folds int32 opaque predicates by tracking which bits of a `const` are known through
  `& | ^ ~ + - << >> >>>` and `Math.imul`, for example `(Math.imul(s | 1, (s | 1) + 1) & 1) === 0`
  when only some bits of `s` are known; a test that the low unknown bits decide is tried for every
  value of them. Every read of the `const` must come after its declaration, and `Math.imul` must be
  unpatched (in the default lane also never reached through an alias of the global object).
  BLA (bla-agents) samples are in `fixtures/bla/`.
- A known-bits seed may take its low bit from a checksum: `typeof f === "function" && f(0) === K`
  is evaluated when `f` is a top-level function called nowhere else whose only outside names are
  top-level literal data (read nowhere else) and intact `Array.isArray`, `Math`, `String`,
  `Number` and `Object` members. On BLA this decides the remaining split-string predicates, so
  `merge-strings` turns each split string back into one literal.
- The builtins scan follows a `const` alias of the global object (`globalThis`, `window`, `self`,
  `global`, or `typeof globalThis !== "undefined" ? globalThis : {}`): the alias is judged like the
  global object itself, so member reads and writes of names that are no built-in pass, and passing
  it on still counts as an escape. `Object.defineProperty(globalThis, "key", ...)` as a statement
  passes when the key names no built-in, global object, code runner or top-level binding.
- The scan's numeric-key fixpoint drops a binding as soon as its evidence fails, so long chains of
  bindings no longer exhaust the pass limit and turn every computed key into an unknown one.
- `new Array(n)` (an integer literal) and `new Array()` count as code that cannot call user code at
  the top of a file, when `Array` is the unbound global and no other input names it, so a
  known-bits seed declared after one is not refused for that reason.
- Loader layers: a script whose last statement hands a computed text to `Function` or `eval`
  (JSFuck, JScrewIt, jjencode, aaencode, or a loader that builds the text through a callback)
  is peeled when the evaluator can run the statements before it and the text-building part
  without observable effects; `Function` calls on the way run exactly. The default lane keeps
  the encoder's setup statements; `--unsafe` drops them when the decoded code does not name
  them. Generated fixtures for each encoder are in `fixtures/encoders/`
  (`tools/encoders/matrix.mjs`), with an EXPECT ratchet for `cargo xtask obf-options`.

### Changed

- The CLI binary no longer carries the core-js fingerprint table (4 MB): core-js (and core-js-pure) is named only with a library collection that holds the `core-js` package (`untwist libraries update`, `--libraries DIR` or a URL), or in a binary built with `--features libraries-corejs`. Without either, core-js code is no longer annotated or named, its unpacked modules are not placed under `node_modules/core-js/`, the report lists no core-js, and `--modern` stops with "core-js fingerprints are not available". With the package in the collection the output is the same as before. The Babel, swc, tslib and regenerator-runtime helper tables stay built in. The Rust library and the wasm build are unchanged.
- `cargo xtask library-collection pack` adds the hand-written core-js and helper tables as curated packages (`"curated": true` in `collection.json`, the table bytes and string keys unchanged, none of the generated-row filters). A curated shard matches like a built-in table (no cluster or shared-shape rule) and is skipped when the same table is already built in; `--no-curated` leaves them out.

### Fixed

- Loader layers keep an `eval` text that declares a name the prefix or the hand-off names (its
  declaration would be hoisted above the prefix), and keep a `Function` text or indirect `eval`
  text that would see CommonJS module bindings (`require`, `module`, a prefix `var`, `this`) in
  place; the same rule now holds for whole-program `Function` and indirect `eval` layers and for
  the `Function` texts the evaluator runs on the way. `--unsafe` keeps the prefix when the text
  reaches it through a `constructor` chain or spells its names in a string.
- `--unsafe` function-to-arrow keeps a callback passed after a spread argument, or to a local
  function that is reassigned or declared twice.

## [0.6.0] - 2026-11-24

There is no 0.5.1: the readability work planned for it ships in this release, together with
library recognition, `--modern`, `--split-assume-modules` and experimental `--types`. The
soundness review of the default lane went on, so some inputs that 0.5.0 decoded or unpacked now
keep that code (with a note) where the rewrite could change what the program does.
Every javascript-obfuscator 5 option combination is now checked before a release: applied once,
it decodes fully in both lanes, and the default lane never gives wrong output.

### Added

- Library recognition: copies of core-js 3.x, regenerator-runtime, tslib, @babel/runtime and
  @swc/helpers are recognised by their exact shape, also when terser inlined or mangled them.
  Their bindings get the library's names in every rename mode, each module gets a comment such
  as `// core-js 3.36–3.41: internals/well-known-symbol.js`, and `--report` lists the
  libraries found (#328, #350, #353, #374).
- Split modules are named after their library (`node_modules/<package>/<path>`), the bundle's
  own module path or their main export instead of a minified name (#377), and the text report
  keeps library modules apart from the app's own (#383).
- Rollup commonjs wrappers: the `requireX()`/`hasRequiredX` lazy wrappers in their terser,
  esbuild and swc forms, rolldown's `__commonJSMin` factories and the commonjs interop helpers
  are named in every rename mode (#296); with `--split-hoisted` each wrapper becomes its own
  module (#300); under `--rename smart` a wrapper call's result is named after the module it
  returns (#311).
- More `--rename smart` evidence: the property a value is read from or stored under,
  constructors, element ids, selectors and storage keys, event and timer callbacks, `fetch`
  responses, boolean returners, PascalCase for functions only used with `new`, and parameter
  names every call site agrees on (#290); long hex tokens it has evidence for (#301).
- `--rename smart` names parameters from how the body uses them (`arr`, `str`, `promise`,
  `event`, `Ctor` for one used with `new`) or from the parameter they are passed on to,
  functions from what they return, call results from their callee, values from the property
  they are read from, and `const v = this` as `self`. A value is never named after a verb such
  as `get` or `mul`, and a reassigned binding no longer keeps its first value's name (#500,
  #501, #502).
- `--modern <baseline>` (opt-in) removes core-js polyfills of features native in an ES year
  (es2017 to es2024) or a list of engines, and turns calls of the core-js helpers left into the
  native form (#317, #374).
- `--split-assume-modules` (with `--split-hoisted`) splits Rollup iife bundles as ES modules,
  also inside an outer wrapper function, and infers module boundaries inside a Rollup entry.
  The tree is meant for reading and for running in node; it does not keep
  `document.currentScript`, classic-script load timing or sloppy top-level behaviour
  (#325, #333, #336). `--split-hoisted` also splits scope-hoisted ES module bundles (#313).
- `--types` (experimental, opt-in) annotates, as JSDoc or TypeScript, only the types that hold
  for every value on every run (#375, #403, #407).
- Unpack: webpack 4 and 5 entries together with the chunks they load (`import()`,
  `splitChunks`, `runtimeChunk`) (#323); webpack's eval devtools, by inlining direct `eval`
  layers that behave the same in place (#305); rolldown and Vite 8 bundles (#299); esbuild and
  bun `--splitting` entries whose helpers live in a shared chunk (#327); esbuild, bun and
  Parcel bundles that other tools reshaped (#306); webpack 4 modules that read the module
  polyfill (#358, #368).
- Untranspile: optional chains lowered with a loose `== null` test (#302), default parameters
  behind Babel's leading declarations (#284), and helper versions from pinned Babel 6 to 8,
  tsc 3.9 to 7.0, swc and esbuild releases (#294, #312). Under `--unsafe`: `for … of` loops
  from Babel's iterator helper, TypeScript's `__values` and the inline iterator protocol
  (#318, #320, #326), Babel regenerator generators and async functions (#324), `super`
  properties (#326), and a `'use strict'` that protection code had pushed out of place (#312).
- Unminify moves a bare `var` to its first use or makes it the declaration (#288, #351), and
  var-to-let also converts vars that later closures read (#319).
- Deobfuscate: javascript-obfuscator applied two to four times in a row (#412) and its
  browser-no-eval and service-worker targets (#424, #442); layers built from array and object
  conversions or a constructor chain (#287). A bytecode interpreter loop beside its bytecode
  gets a report note (#298).
- When the default lane leaves a string array undecoded because the program may replace a
  built-in, one warning (also on stderr) says to rerun with `--unsafe` (#289).
- Playground: the settings the CLI had and it did not (modern, split hoisted, assume modules,
  inline source maps, budgets) (#360); opt-in library names with a libraries panel and folded
  library modules (#286, #330, #342, #345); live run progress (#342); editor themes, a site
  theme and GitHub Dark as the dark default (#367, #372, #379). The output scrollbar stays in
  its pane on wide lines (#344).

### Changed

- Krunker 10.0.7 reads much closer to its source: under `--unsafe`, 444 Babel classes become
  `class` again (none in 0.5.0) (#312), optional-chain rewrites go from 94 to 208 (#302), the
  `requireX` wrapper names are spread to the values they return (#311), and the walls of bare
  `var` at the top of functions are gone (#351). With `--rename smart`, names of two characters
  or fewer fall from 35% to 27% of its bindings (#500, #501, #502).
- The default lane keeps every function and class name (#399) and never renames a classic
  script's top-level bindings (#385).
- The playground finds refusal places in one pass per line and lists each place once, capped
  at 1,000 (game3.js: 80 s and a 5.8 MB report before) (#339, #341); language features work on
  outputs up to 32 MiB (#295). Deep control-flow nests and the init-order scan cost less
  (#283, #293).
- The playground takes inputs nesting up to 1,532 units deep instead of passing through
  those over 661, so Krunker 3.2.1 and 3.2.4 run there as in the CLI (#304).
- New release checks: a bundle-variant matrix over webpack, Rollup, Vite, rolldown, esbuild,
  bun and Parcel versions (#297), outputs of obfuscators other than javascript-obfuscator
  (#285), and the javascript-obfuscator option sweep: every option combination, also applied
  two and three times over (#444). `RELEASING.md` lists the matrices to run before a release
  (#348).
- `docs/known-divergences` states the default-lane premise for reading a function's caller
  (#415), the `--unsafe` premise for writes to decoder names (#428), and that no
  `Object.prototype` property may be named like a module id or request (#386).
- Pull requests link their playground preview (#370).

### Fixed

#### Default lane soundness

- The check for a patched built-in is an allow-list of safe forms and fails closed by
  structure: method results, sloppy `this`, timers on any receiver, code-runner modules,
  window-alias writes, escaping strict functions, numeric keys, RegExp statics and nested
  objects' built-ins are judged, and only literal keys are trusted (#356, #359, #363, #418,
  #419, #422, #425, #434, #447, #454, #459, #463, #473). Such programs keep their decoder
  calls.
- Every route to a code runner (`eval` however it is bound, `Function`, code text that may
  hand back a runner, reflective readers and writers) is closed by an allow-list (#390, #405,
  #409, #413, #414, #421, #432), and uses of the global object and namespaces are judged
  against a closed list of safe forms (#423, #437, #453, #457).
- When several inputs run together, a name counts as shared only when another input may reach
  it, and shared inputs, foreign values and built-in names another input declares are judged
  across all of them (#331, #381, #388, #445, #462, #465). Getter, descriptor and
  `Proxy` copies carry their source into these checks (#435).
- Top-level decoder aliases and wrappers are judged by the global reach check, and machinery
  names on shared globals stay under observation (#427, #436). Self-defending code is kept
  when a built-in it calls is used as a value or its check would see a reprinted function,
  and only obfuscator.io's own call controller shape is removed (#308, #337, #417, #426,
  #431, #433). Console silencing and debug-protection timers are kept together, and the
  default lane keeps its output when only some protection layers can be removed (#443, #448).
- Decoding refuses or keeps more shapes it cannot prove: a stateful decoder unit where any call
  throws, a decoder that rewrites itself or is written outside its body, memoised calls that
  touched the memo, string-array wraps in loops or over non-plain bindings, Annex B block
  functions and lexical aliases in folds, method calls on assembled objects and optional calls
  in dead-branch proofs, and the meaning of statements a cookie or checksum wrap moves (#450,
  #458, #460, #466, #467, #468, #471, #472, #483, #484, #486, #487, #488, #489, #490, #491,
  #492, #493, #494).
- A lazy decoder's aliases are replaced only when its inner function behaves like a fresh call
  of the outer one and every copy of the decoder goes into a binding that is only called and
  that no parameter, export, `eval`, `with` or other input sees; any other copy keeps every
  alias (#497, #499, #503, #505).

#### Decoding coverage

- Layered and re-obfuscated javascript-obfuscator output decodes much further, mostly under
  `--unsafe`: detection and decoding run again until a round removes nothing, with refusals
  and source maps carried across rounds (#441, #446, #474); layered stateful units, string arrays
  hidden by later layers and dead-code copies, inlined rotators and inner-decoder calls
  (#438, #440, #452, #461, #470, #481); debug protection, console disabling, call controllers
  and self-defending that later layers kept or moved into decoders and wrappers (#449, #455,
  #469, #475, #476, #478, #480, #485); control-flow objects that hide the decoder, alias each
  other, form cycles or escape only in dead branches (#439, #456, #464, #477, #479, #482);
  nested objects used only in dead branches (#451, #456). Earlier fixes: a literal
  control-flow nest's inner call (#309), storage objects that escape only in dead code (#292)
  and a minified decoder beside its rotator (#376).

#### Unpack

- Unpack refuses more bundles the written tree would run differently: browserify bundles
  whose `require` is redeclared, renamed or unlinkable, or whose requests or host values the
  written files answer differently (#310, #316, #335, #384, #387, #393, #408); webpack shapes
  a merged tree runs differently (#338, #361, #382); every bundle kind on a `.caller` or
  `.arguments` read (#365); a split where a module could see its caller (#391); host
  `require`, `import()` and node's loader outside an allow-list (#401). Strictness, early
  errors, node's module goal and UMD `define` follow the source (#395, #398, #406, #416).
- Because a factory may now use the host's `require` only with `typeof` or a literal request
  (#393), the obfuscated webpack apps whose code aliases it (`const r = require`) are no longer
  unpacked under `--unsafe`: 20 obf-matrix webpack cells keep the bundle packed.
- `--split-hoisted` no longer splits a webpack runtime chunk (#307), and real strict webpack
  5 bundles split again (#397). Factory parameters are renamed with every same-named binding
  moved aside (#410), twice-declared functions and setter parameters work in assumed-module
  runs (#347), and factories with a closed `eval` unpack (#354). When a bundle leaves some
  inputs out, each is written under its own name (#291).

#### Unminify, untranspile and rename

- Names: `NamedEvaluation` names survive every naming-slot rewrite (#362); a function assigned
  to a parenthesised name stays unnamed (#349); catch parameters, catch vars and Annex B block
  functions keep their bindings (#400, #402, #420, #430); rename keeps names that code text
  spells, `with` and `eval` see or computed keys and reflection read (#352, #378, #389, #394,
  #411).
- Unminify keeps a `var` a `var` when a catch body redeclares it or beside a direct `eval`
  under `--unsafe` (#303, #396), and splits a function out of a sequence only when no name is
  read (#314). Lexical untranspile, layer inlining and `--modern` close the gaps their reviews
  found (#332, #340, #343). Further review findings on eval, `with`, class awaits, script
  globals and Annex B hoists are fixed (#334, #346, #355, #364, #369, #380, #392, #404).
- A shorthand property keeps its key when rename, unpack, inlining or decoding changes its
  value, so `({x} = o)` and `{__proto__}` keep their meaning, and `__assign` over a literal with
  a `__proto__` key is refused (#496, #498, #504).

### Known gaps

The javascript-obfuscator sweep decodes every single-layer case fully in both lanes (655 of
655). Over all 2,189 run cases, the default lane gives no wrong output and `--unsafe` fully
decodes 2,180. The nine left are multi-layer (re-obfuscated) inputs that keep machinery under
`--unsafe`: a web of about 30 control-flow objects whose escapes depend on each other (pw038-n3,
console disabling left); a debug interval recognised only after the rotator is refused
(pw023-m3, debug protection left; a second run decodes it); decoder aliases read before their
declaration or a decoder used other than as a callee (rnd001-m3, pw039-m3 twice; the functions
case prints differently); a later layer's `renameProperties` or control-flow flattening inside
the console-disable callback (pw035-m3, pw038-n2); and a stateful decoder whose call throws in
the first pass (rnd022-m3, rnd031-n3). On all nine the default lane prints what the obfuscated
input prints, and pw035-m3 and rnd031-n3 also print like the source under `--unsafe`. Outside
the sweep, a dead-code copy that calls the wrapper with a bad index leaves the string array
under `--unsafe` (the default lane prints like node).

Two hand-written shapes found in the last review round are not fixed yet. A classic script
that copies the lazy decoder as `function t() {}; t = d`, an Annex B block function, a
`for (var t = d; …)` init, a labelled assignment or `var t = t0.k`, and reads the copy back
through `this.t` or `globalThis`, still has its alias calls redirected, so the copy compares
equal to the decoder where node says it does not (both lanes; `var t = d` is right). In a
multi-input run under `--unsafe`, a whole-program `eval` layer is replaced at the top level,
so its `let`, `const` and `class` become script bindings that another input can clash with or
read. No javascript-obfuscator output in the sweep has either shape.

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
- `--rename smart`, evidence-based names that keep program behaviour: machine-made bindings are
  named from what the code shows (loop counters, callback parameters, `catch`, `require` and
  `import`, destructured keys, the `createElement` tag and the like), under the same soundness
  guards as the other rename modes. The playground offers it too (#279).
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
- `docs/known-divergences/deobfuscate.md` lists the shapes where the default lane still does
  not take a value for the global object (#281).
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
- A string array is no longer decoded when the program may replace a built-in through an
  element of a host call's result or through a function the `Function` constructor builds
  from code text (#280).
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
