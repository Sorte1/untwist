# Known divergences from the reference deobfuscator: Unpack

Untwist's unpack stage is verified against the reference deobfuscator by `cargo xtask diff-unpack`; the items below are deliberate, not bugs.

- Module paths are stored in one normalised POSIX form with **no `./` prefix** (`3.js`, `index.js`, `node_modules/lib/index.js`), so `bundle.json` reads `"path": "3.js"` where the reference deobfuscator writes `"path": "./3.js"`. The file names on disk are identical, and `ModulePaths::specifier` re-adds the `./` when a specifier is emitted, so a generated `require("…")` or `import … from "…"` is byte-identical to the reference deobfuscator's except where the next item says otherwise. The reference deobfuscator is itself inconsistent here: browserify entries are already stored as `index.js` with no prefix while unreachable browserify modules keep `./<id>.js`.
- A specifier for a file under the root `node_modules/` is the bare form (`require("lib/index.js")`) only when node would resolve it to that file. The reference deobfuscator always writes the bare form. Untwist writes the relative path instead (`require("../node_modules/lib/index.js")`) when a file or directory at `<ancestor of the requester>/node_modules/<the same tail>` is also written (node searches those first, and tries `.js`, `.json` and `.node` after the name), when the tail has the shape of a node core module name (`events`, `fs/promises`, anything starting with `node:`; node loads core modules before it looks at `node_modules`), when it starts with `#`, and when the target is not a path the bundle assigned. A sibling path that starts with a dot (`.config/a.js`) gets its `./` too, where the reference deobfuscator leaves a specifier node reads as a package name. All linkers go through `ModulePaths::specifier` in `specifier.rs`; the webpack reference-parity lane keeps the reference deobfuscator's form.
- An AMD or UMD factory that declares a function with the name of one of its parameters (`define(["dep"], function (a) { function a() {} … })`, also inside a block) is refused and the input is left as it is. The lowering turns each parameter into `var a = require(…)` at the top of the module, where the hoisted function would be overwritten by the dependency; in the factory the function wins. The reference deobfuscator does not unpack AMD or UMD.
- `bundle.json` writes `"entryId": null` when no entry module was found; the reference deobfuscator writes the empty string. `entry_id` is an `Option<String>` internally rather than a `''` sentinel.
- `bundle.json` carries two fields the reference deobfuscator does not: `isEntry` per module, and a `chunkIds` array. The reference deobfuscator captures chunk ids in its JSONP matcher and then discards them.
- Cross-chunk requires that cannot be resolved are reported as diagnostic notes (`module <id>: require("<id>") has no module in this bundle`) instead of being injected into the output as a `/*the reference deobfuscator:missing*/` leading comment. `xtask::diff::normalize` prints with comments disabled, so this is a non-difference in the differential test.
- A module id that would escape the output directory (`../tmp.js`, `./\..\node_modules\debug\src\index`) is replaced at path-assignment time with `_unsafe_<sanitised id>.js` plus a note. The reference deobfuscator keeps the traversing path and throws inside `save()`, losing the whole bundle. Two modules that resolve to the same output path are uniquified with `-2`, `-3` stem suffixes and a note; the reference deobfuscator does not detect path collisions at all and silently overwrites.
- Extracted module programs start with a **script** source type, so a module is ESM only after its own `require.r(exports)` call. Untwist's parser tries `SourceType::mjs()` first and keeps it whenever a file has no errors, so inheriting the bundle's source type would mark every module ESM.
- Browserify's `./utils` versus `utils/index.js` ambiguity is resolved by constraint: a module whose path came from an extensionless relative request is moved to `<stem>/index.js` when its own relative requests escape the tree from `<stem>.js` but not from `<stem>/`. The reference deobfuscator marks this case `test.skip` and always produces `utils.js`. The inference is conservative and leaves all three passing `path.test.ts` cases and all four browserify fixtures unchanged.
- Browserify modules are linked through their dependency maps. Each free `require("<request>")` whose request is a key of the module's map is rewritten to the path of the file of the mapped id, so two versions of one package, a bundled shim named like a node core module (`events`, `buffer`) and a module reached by two requests from different directories each load the file the bundle loads. The reference deobfuscator keeps the request as written and relies on the inferred file paths, and so does the reference-parity lane. A computed request, a request outside the map and a local function named `require` are left as written; a request outside the map that names a module id, and a renamed `require` that is still called after linking, are reported as notes. When the bundle lists several entries (`[1,7]`), `index.js` ends with a `require` of each later entry in order; when the entry module may return early or binds `require` itself, the later entries are not run and a note says so. The reference deobfuscator runs only the first entry.
- Several input files are detected separately and their module maps merged into one bundle before paths are assigned, so a require that crosses a chunk boundary resolves. The reference deobfuscator has no multi-file API; every cross-chunk require becomes a missing marker.
- User path mappings are a closed enum of four shapes (string literal, numeric literal, identifier, `object.property` member) rather than arbitrary `@codemod/matchers` matchers; a regexp-literal matcher is not implemented in v1. A mapping key that matches a second module is skipped and the first module keeps it, where the reference deobfuscator throws `Mapping X is already used.` Mappings that matched nothing become notes; the reference deobfuscator tracks them in a variable it never reads.
- Webpack helpers nobody undoes (`require.t`, `require.e`, `require.S`) and forms this crate did not match (a surviving `require.d`, `require.r` or `require.n`) are reported as notes. The reference deobfuscator leaves them in the output silently.
- In the conservative comparison mode, `convert_default_require` resolves the `require.n` argument and the getter variable by **name** over the whole module program (first `require(<number>)` declarator wins) rather than through scope bindings. A module that shadows one of those names in an inner scope would diverge; webpack's generated names make that unreachable in practice.
- **`require.n` becomes `x.default` only where the target is proven harmony.** The reference deobfuscator turns `var g = require.n(x)` into `var g = x.default` when the required module calls `require.r` anywhere and into `var g = x` otherwise, for numeric ids only. That reads the export once, at the declaration (a circular import or a late `require.d` getter then gives `undefined` where `g()` gave the value), trusts a `.r` on another object or one followed by `module.exports = ...`, and leaves `g()` and `g.a` calls on a string-id bundle with nothing to serve them. By default Untwist rewrites only when the required module is already ESM or starts with `require.r(exports)` and never names `module` or rebinds `exports`/`require`, the getter is declared directly after its `require(id)` (string or numeric id), neither binding is reassigned or redeclared, and every use is `g()` or `g.a`; each use then becomes `x.default` (`(0, x.default)(...)` where it was called) and the getter declaration is dropped, so the export is read when the bundle read it. A module that hands the runtime away (`require` passed on, `require.c`, `require.m`, `eval`, `with`, the factory's `arguments`) stops the rewrite for the whole bundle. Everything else keeps `require.n`, served by the bundle's own helper in `_runtime/webpack.js`. The conservative comparison mode keeps the reference deobfuscator's conversion; the default snapshot of `fixtures/reference/unpack/webpack-esm.js` and `default_require_sees_the_esm_flag_convert_esm_set` were re-pinned.
- **A local or parameter named `require` is not the runtime.** A by-name rewrite of `require(<id>)`, `require.n(...)` and `require.t(...)` also rewrites `function load(require) { return require("./utils") + require(7); }` inside a module, whose arguments then become module paths the function was never given. Untwist asks one question, `scan::is_free`, in the require-path rewriter, the `.n` conversion, the `.t` conversion and the host-`require` shield: a reference that scoping resolves to a binding of the module is left alone, and a node without a reference (one Untwist built itself) counts as the runtime. This holds in both modes, so the conservative comparison mode differs from a by-name rewrite only inside a module that declares its own `require`.
- **A webpack var injection is spliced only where it is proven.** The reference deobfuscator turns every module statement of the form `(function (a, b) { ... }).call(this, x, y)` (or `.call(exports, ...)`) into `var a = x; var b = y;` followed by the body. That changes the program when the call is not the whole module (its declarations join the module's, a `return` now ends the module), when the counts differ, when the body has a directive, a `return` at its own level, `arguments`, `new.target` or `eval`, when a name the body declares at its own level collides with a parameter or with `module`/`exports`/`require`/`__filename`/`__dirname` (a function declaration is hoisted above the new `var` and loses to it), when an argument reads a parameter name or a name the body declares, or when the receiver is `exports` and the body reads `this`. By default Untwist splices only webpack's own wrapper: an anonymous plain function called as the last statement of a module whose other statements are `require.r(exports)`, with as many arguments as plain, distinct parameters and none of the cases above. Any other call stays as written. The conservative comparison mode keeps the reference deobfuscator's splice.
- **A webpack or browserify bundle is taken only when every factory is closed.** The reference deobfuscator finds these bundle shapes anywhere in the program and lifts each factory body into its own file. A factory that reads a binding of the scope around the bundle (a variable beside a nested bundle call, a parameter of the function around it, the `__WEBPACK_EXTERNAL_MODULE_x__` parameters of a `libraryTarget: "umd"` wrapper with externals, a name of the bundle's own prelude), its own `arguments`, a parameter the runtime never passes, or that calls `eval` directly or holds a `with` statement (the module rewrites cannot be proven inside one), then runs in a file where none of that exists. By default Untwist checks every factory with the rule the other bundlers already follow (`closed.rs`: only globals, the mapped parameters and the factory's own declarations) and, when one fails, takes no bundle from that shape: the program is output whole, with the note `module <id> is not closed over its own scope; no bundle is written: it reads <name>, a binding of the bundle's scope`. A later shape in the same file whose factories are closed is still taken. A UMD wrapper with externals is refused this way for now rather than having its externals turned into requires. The conservative comparison mode keeps the reference deobfuscator's behaviour.
- **A webpack runtime is rewritten only when it is proven to be webpack's.** The reference deobfuscator finds the require function by its `modules[id].call(...)` dispatch and keeps the prelude's `require.x = ...` statements, dropping every other prelude statement. By default Untwist first checks the prelude: the require function must be webpack's caching form (the cache lookup, `cache[id] = { exports: {} }` with optional `i`/`l`/`id`/`loaded`, the dispatch, the loaded flag, `return module.exports`); every other statement must be a declaration with a value that runs nothing, a `require.x = <value that runs nothing>` definition (at the top level or in a plain zero-argument wrapper, at most once per name), one of webpack's `__dirname` guards, one entry call (`require(require.s = id)`, a single top-level `require(id)` or the webpack 5 inline entry), or `module.exports = <the entry's exports>`. The require function is used only as a callee, a member object, `typeof` or the receiver of `.bind`, and a helper is never written anywhere but its definition. A helper that reads a binding from around the bundle, the prelude's `this` or `arguments`, `new.target` or `import.meta`, or the host `require`, is not moved. A module that reads or writes a member of its require function that is not a moved helper, passes the require function on, or reaches it through a computed key, means no bundle (`module <id> uses its require function in a way the unpacked files cannot share`). `require.d`/`.r`/`.n`/`.o`/`.t` get webpack's meaning (exports, interop) only when the bundle's own definitions are webpack's recorded texts (`webpack/forms.rs`); otherwise the modules keep the calls, served by the bundle's own helpers. Any other prelude gives the note `the prelude is not proven to be webpack's runtime; no bundle is written: <why>`. Two inputs whose runtimes define a helper differently are not merged (`two webpack runtimes differ`); a module id two chunks define with different text is not merged (webpack keeps the last one loaded); a JSONP chunk whose third element is not webpack 4's deferred-entry list is refused, and a chunk with deferred entries is not merged with a runtime. The conservative comparison mode keeps the reference deobfuscator's behaviour, except for the chunk merges.
- Detection never mutates the input program — module bodies are `CloneIn`-ed — so unpacking no longer has to run after codegen, and a webpack 5 bundle sitting at the top level of a file with no IIFE wrapper is detected, which the reference deobfuscator's `BlockStatement`-keyed visitor cannot see. The webpack 5 container search also backtracks over candidate declarations and requires a non-empty container, so a `var __webpack_module_cache__ = {}` declared before the real container no longer defeats detection.
- An expression-bodied arrow is never accepted as a module factory (the reference deobfuscator builds `t.program(undefined)` from one and crashes), and a var injection whose parameter has no matching argument produces `var x;` instead of crashing the reference deobfuscator's template.
- `-o DIR` with `--force` deletes only an explicit list of files: `deobfuscated.js`, `bundle.json`, the files the current run is about to write, and the module paths recorded in an existing `bundle.json`. No directory is ever removed and nothing recursive happens; the reference deobfuscator's CLI clears the whole output directory recursively.
- Object literals with a single property print on one line (`var x = { Console: bar };`) because `oxc_codegen` only goes multi-line above one property, where Babel always breaks. This is a printer difference and disappears under `xtask::diff::normalize`.
- A user path-mapping key is a literal path exactly when it starts with `.` (not only `./`): `resolve_mapping_path` in `mappings.rs` refuses `../escape.js` as unsafe instead of turning it into `node_modules/../escape.js`, because `PosixPath::parse` rejects any normalised path starting with `../` regardless of how the key was written.
- A module whose derived or resolved path is refused by `PosixPath::parse` gets the `_unsafe_<id>.js` fallback from `fallback_path` plus a `Note` (`module <id>: unsafe path <source> replaced by <path>`) from `assign_paths`, and this holds after a chunk merge retags a module's entry id too: `Module::assign_default_path` is the single place that derives a path, applies the fallback and records `unsafe_path_source`, so `Module::new` and `merge_bundles`'s retagging both go through it and both re-emit the note.
- `-o DIR` refuses to write or `--force`-delete through any symlink component below DIR (DIR itself may be a symlink): `writer::symlink_free` walks every path segment below DIR with `symlink_metadata` and refuses if any is a symlink, and `writer::is_contained` canonicalizes the deepest existing ancestor of the target and requires it to stay under DIR; both `write_all` and `clean` (the `--force` deletion path) are gated by this and report each refusal instead of writing or deleting through it, excluding it from `files_written`. The reference deobfuscator's CLI only throws on lexical `..` traversal at save time and has no symlink awareness at all.
- The `symlink_free`/`is_contained` containment check is check-then-act, not atomic: an attacker able to write inside DIR while `untwist -o DIR` is running can still race it. `write_all` writes each file through a same-directory temp file and `rename`, so a hard link planted at the target path no longer leaks the write into the link's other name (`rename` replaces DIR's own directory entry and leaves the other name alone), and a symlink planted at the target's own leaf position isn't followed by `rename` either. The residual gap is an intermediate path segment (a directory component, not the leaf) that is swapped for a symlink between the check and the write: `NamedTempFile::new_in` and the following `rename` both re-resolve that segment at call time and would follow it. Closing this fully needs `O_NOFOLLOW`/`openat`-based path resolution, which is out of scope here.
- **The webpack entry marker is `.s`, and nothing else.** The reference deobfuscator matches only
  `__webpack_require__.s` (`unpack/webpack/common-matchers.ts:143`); its own closed-unmerged PR #34
  proposed adding `.j`, but `.j` is webpack 5's `RuntimeGlobals.chunkId` — the id of the chunk currently
  executing, never an entry module id — which is exactly why that PR was closed rather than merged:
  `fixtures/reference/unpack/webpack-5-no-entry.js` assigns `__webpack_require__.j = 377` for
  `installedChunks['377']`, and `377` names no module. Untwist rules `.j` out alongside the issue's other
  four names — `.e`, `.d`, `.m` and `.r`, the ensure-chunk, define, module-table and `__esModule`-setter
  helpers — and pins the collision case (a chunk id that happens to equal a real module id) in
  `fixtures/unpack-gaps/webpack-5-chunk-id-j.js`, so a chunk id is never read as the entry even when it
  matches one.
- **A startup body with several top-level `require(id)` calls is refused.** The reference deobfuscator
  takes the first as the entry silently (`common-matchers.ts:157-164`), which is what reference deobfuscator
  issue #68 objects to, and the written files then run only that module. By default Untwist refuses a prelude
  that starts more than one entry module (several top-level `require(id)` calls, or one beside a webpack 5
  inline entry IIFE): `the prelude is not proven to be webpack's runtime; no bundle is written: the prelude
  starts more than one entry module`. A single top-level `require(id)` is the entry in webpack 4 and 5. The
  conservative comparison mode keeps the first id and the note naming the count
  (`Bundle::entry_candidates`), so the uncertainty is visible there.
- **A Promise-based chunk-loading runtime's entry id now comes from its `.O` registration.**
  webpack 5's `output.chunkLoading: "require"` runtime (`__webpack_require__.O`/`.f.require`
  against an `installedChunks` map, reference deobfuscator issue #33) still pushes its modules through the same
  `self["webpackChunk…"]` global the JSONP detector already matches, so measuring against
  `fixtures/unpack-gaps/webpack-5-promise-chunk.js` found the modules and chunk ids correctly and
  only `entry_id` came back `None` — nothing read the `.O(0, [chunk], () => require(id))` call that
  names the chunk's requested module. `registered_entry_id`/`required_id` in `chunk.rs` read it from
  the first such registration anywhere in the program's top-level statements (unwrapping a top-level
  `SequenceExpression`'s elements, since minifiers chain several `.O(...)` calls with the comma
  operator, and following both arrow-body forms — expression and block — since minifiers emit
  either).
- **the reference deobfuscator's closed-unmerged PR #50 shapes are fixtures now.** Its two webpack 4
  shapes are built by webpack 4.47.0 from the shared app: `fixtures/unpack-bundlers/webpack/webpack4-dev-paths.js`
  (readable `./app/…` ids, `__webpack_require__.t` around a JSON import) and `webpack4-externals.js`
  (a factory calling the host's own `require("path")`). Both unpack, and the written modules run
  under node with the bundle's output. The host `require` stays the host's (see below);
  `require.t(id, 9)` becomes `require(id)`; every other `.t` mode, and webpack's `.r/.d/.o/.n` where a
  module keeps them, are taken verbatim from the bundle's own prelude into `_runtime/webpack.js`
  behind a synthesized `__webpack_require__(id) = require(<path>)`.
- **the reference deobfuscator's ESM conversion runs only where it is proven.** The reference deobfuscator
  turns every harmony module into ESM: a numeric harmony import of a CommonJS target becomes
  `import * as x` (the namespace lacks the target's properties), a `.d` on a namespace object
  becomes `export const` (the namespace stays empty), a getter becomes an eager `export let`, and
  imports hoist above earlier effects. By default Untwist writes `export`s only for a module whose
  conversion is proven (strict, harmony, `.d` only on `exports` with getters reading distinct
  top-level bindings, no leftover `exports`/`module`/`this`/`arguments`, no require cycle, no
  `__esModule` observation, no interop target, every consumer a plain member read) and keeps its
  `require`s, so evaluation order is unchanged; every other module stays CommonJS with its helpers
  in `_runtime/webpack.js`. The webpack 5 inline entry is written as `index.js` under the same
  rules. The conservative comparison mode keeps the reference deobfuscator's conversion, so `diff-unpack` is unchanged; several default
  the reference deobfuscator snapshots were re-pinned for it.

- **A factory parameter is not renamed onto a name its body already reads free.**
  the reference deobfuscator renames webpack's `(e, t, i)` and browserify's factory parameters to `module`,
  `exports` and `require` even when the body also reads the host's own `require` (or `module`,
  `exports`): `require.hmd(e)` beside `i.d(t, …)` becomes `require.hmd(e)` beside `require.d(…)`,
  so the host's reference is captured by the runtime's and node sees a different program
  (`crates/untwist/tests/host_require.rs` runs both). Untwist gives such a parameter a fresh name
  (`_require`) instead; the conservative comparison mode keeps the reference deobfuscator's fold, so `diff-unpack` is unchanged. The default
  snapshot of `fixtures/reference/unpack/webpack-esm.js` was re-pinned for it.

### Bundlers the reference deobfuscator does not unpack

The reference deobfuscator detects only webpack and browserify (its issue #117). Untwist also splits the bundlers
below. Every kind follows one contract: a written module set must behave like the bundle under
node, or no bundle is written and the whole program is output as before, with the note
`unpack: <kind> bundle shape found but not proven; no bundle is written`. A refusal is never a
failure; the shapes each kind accepts and refuses are pinned by `fixtures/unpack-bundlers/`
(generated by the real bundlers at exact versions, plus hand-written `min-`/`neg-` shapes) and run
under node by `cargo xtask diff-unpack-behaviour` in default and unpack-only modes.
`bundle.json`'s `"type"` names the kind in lower case (`esbuild`, `rollup`, `systemjs`, `amd`,
`umd`, `parcel`, `metro`, `turbopack`, `closure`).

- **Detection order.** The reference deobfuscator's four detectors run first, unchanged, then Closure's module
  manager, SystemJS, esbuild (Bun's wrapper unwrapped first), Metro, Parcel and Turbopack, then a
  UMD/AMD/SystemJS wrapper's single module is run back through the splitting kinds (an esbuild
  bundle inside UMD splits), then AMD/UMD itself. Rollup/Vite scope hoisting runs last and only
  when asked (below). The new probes look only at the top-level statements and one wrapper body,
  so a non-bundle input is not re-parsed.
- **Output form.** A CommonJS factory becomes a module with its own body and CJS names; an
  ESM-shaped module (SystemJS, a Rollup split) is written as ESM; esbuild's lazy `__esm` init stays
  a lazy exported `init_x` function, because lazy-once is not import hoisting. A path-like id the
  bundler recorded (esbuild keys or `// path` comments, SystemJS names, AMD ids, Parcel and
  Turbopack asset paths) becomes the module's path through the usual traversal and collision
  checks; otherwise `<id>.js`. The entry is `index.js`, reserved even against a module whose own id
  is `index` (that module gets `index-2.js`).
- **The runtime module.** A helper whose semantics are not `require`'s (esbuild's
  `__toESM`/`__esm`, webpack's `.t`, Parcel's hoisted helpers) is moved verbatim from the bundle
  into `_runtime/<kind>.js`, which the modules that use it require; it must itself be closed (only
  globals and other helpers free), hold no `with`, and read none of the host names below, which
  every written file has for itself. A moved esbuild binding that is assigned after its declaration
  (the `var` an `__esm` init fills) is exported through a getter, and the modules read it as
  `runtime.name` each time, a call as `(0, runtime.name)()`; a bundle that deletes such a name,
  assigns it outside the runtime module or names it where only a binding fits (an `export { name }`,
  a `for (name in …)` target) is not unpacked, and two inputs whose runtime exports such a getter
  are not linked into one bundle. esbuild's lazy helpers, the names getter they call and the
  `__require` shim are trusted only while each is declared once and never assigned. Metro's `importDefault`/`importAll` live inside Metro's polyfill
  closure, so `_runtime/metro.js` holds a synthesized `metroRequire(id)` plus copies of Metro's two
  interop functions. Closure's shared `var`s live in a null-prototype holder,
  `_runtime/closure.js`.
- **Free host names are never renamed, and never captured.** A factory that reads a free
  `require`, `module`, `exports`, `__filename` or `__dirname` the bundler did not map is refused
  (except a bare `require("pkg")` of a host package, which is kept as the host's), since the
  written file would see a different value.
- **Wrapper names a module declares itself are renamed.** A factory body (or an unwrapped entry)
  that declares `require`, `module`, `exports`, `__filename` or `__dirname` itself, where that name
  is not one of the factory's own parameters, gets the binding renamed (`_require`, ...) in the
  written CommonJS file: node's wrapper binds those names, so a `let` would no longer parse and an
  uninitialised `var` would read the wrapper's value instead of `undefined`.
- **esbuild's `__require` shim in an ES-module bundle.** Without the `createRequire` banner the
  shim finds no `require` and throws "Dynamic require ... is not supported"; such a bundle is not
  split, since a written CommonJS file would load the module instead.
- **Rollup/Vite scope hoisting is opt-in: `--split-hoisted`.** A hoisted bundle has no module
  boundaries of its own, so the split is a heuristic over the wrapper's top-level declarations
  (clusters by reference adjacency, a binding's writers kept with its owner). It is refused for a
  sloppy wrapper, any top-level statement besides the wrapper, any `currentScript` read, and any
  binding written across a module edge. `game.js` reads `document.currentScript` and is therefore
  not split. A function declared more than once at the wrapper's top level keeps its last
  declaration only, the one every reference in the wrapper sees; a name declared twice in any other
  way (a `var` beside a function) is refused, since an ES module may not declare a name twice.
- **`--split-assume-modules` (with `--split-hoisted`) assumes the split tree is ES modules for
  reading and for running in node.** It does not preserve `document.currentScript` (an ES module
  sees `null`), classic-script synchronous load timing, or sloppy-mode top-level semantics. With
  it, Rollup's iife `import.meta.url` stand-in
  (`var x = typeof document != "undefined" ? document.currentScript : null`) no longer refuses the
  split and stays in the entry as written, and the bundle may sit one function below the program
  top level, inside an outer wrapper whose other statements are `var` declarations (and, when the
  outer function is strict, function declarations and code after the bundle). The plan is not the
  heuristic: every @rollup/plugin-commonjs `requireX` wrapper, with its flag and its exports
  holder, becomes its own module; Rollup's own helpers that those wrappers read
  (`commonjsGlobal` and other side-effect-free declarations) go to `chunk_shared.js`; everything
  else stays in the entry, in order. A wrapper module only holds declarations that run no code
  (function declarations, `var`s with no value or a plain literal, object or array value), so
  importing it before the entry runs changes nothing; a value is only moved when nothing can read
  it before its original line. A wrapper that reads code left in the entry, is written from
  outside, or would be shadowed by an outer-wrapper name stays in the entry. Outer-wrapper
  bindings a wrapper module reads come from a generated `shell.js` that exports them as live
  bindings: they must be declared without a value, the bundle may not write them, and the outer
  code may assign them only after the bundle with `x = e;` or `var x = e;` statements, which the
  entry routes through a setter exported by `shell.js` (`set_x(x = e)`). Under the flag the
  lexical stage keeps `var`s that later closures read (the split reads Rollup's `var` holders), and
  once the bundle is split, `var`-to-`let`/`const` runs again on each written module, where each
  module is its own scope; `deobfuscated.js`, written beside the tree, keeps those `var`s. Without
  the flag nothing changes.
- **Closure.** Only the module manager's `try {…} catch (e) { X._DumpException(e) }` segments
  are split (ids from `/*_M:id*/` markers or begin/end loader calls checked against the
  `_ModuleManager_initialize` graph). The namespace object is read once, in
  `_runtime/closure.js`, and every file gets that one object as its wrapper parameter, as the
  bundle binds it once; a segment that assigns or deletes the namespace property on the global
  object (`window._ = …`, `this._ = …`) is refused. A `var x = this` line of the wrapper is copied
  only into the files that name `x`. Closure ADVANCED property renaming stays a non-goal. A Closure
  bundle inside a UMD or AMD factory is not re-fed: its top-level `this` is the factory's receiver,
  which an AMD loader sets to `exports`. Every written segment runs with `globalThis` as its
  top-level `this`, which is what the bundle's `this` is in a browser classic script. This is an
  accepted assumption: required as a CommonJS file under node the bundle's `this` is
  `module.exports`, so there a bundle that reaches globals through `var window = this` and its
  output differ.
- **esbuild.** A bundle is split only when every `__commonJS` factory is declared before any
  statement at its level can run code: up to the last factory declaration only helpers, function
  declarations, imports and `var`s whose values are functions or plain literals may stand, so no
  `require_x()` can run while `require_x` is still `undefined`. The lazy helper may not bind a name
  that its body reads (`function mod()`, a `catch (mod)`, a parameter named `eval` or `arguments`,
  the names getter as a parameter); the keyed form needs a free `Object`, and a factory keyed
  `__proto__` is refused. A factory in method form that uses `super` at its own level is refused,
  since the written module is no longer a method.
- **Metro.** Factories with 5 or 6 parameters (Metro < 0.52) are refused. `__d` and `__r` are
  Metro's only when the file shows it: the prelude must hold Metro's `require.js` polyfill, equal
  after α-renaming to a recorded text (`metro/runtimes/`, metro 0.87.2 production, not
  minified, as written and as the default and `--unsafe` passes leave it), `__METRO_GLOBAL_PREFIX__` must be bound once to the prefix the calls use, no other
  prelude statement may spell a name ending in `__d` or `__r`, and the statements after the prelude
  may call nothing at their own level but those two. A prelude that defines the names any other way
  is refused, a minified or older runtime included, until its text is recorded. A file with no
  prelude (the names are free, as in a split or lazy bundle) is refused in the safe lane and
  unpacked with `--unsafe`, which takes the names on trust. A prelude polyfill that replaces the
  loader without spelling its name (through a computed key) is not seen. Module ids keep their type,
  as in Metro's `Map`: every definition, dependency entry and `__r` target must be all numbers or all
  strings, and an id `__proto__` or `index` is refused. A literal `require` argument of the other type
  is left to `metroRequire`, whose table is a `Map` and which throws Metro's "Requiring unknown module"
  for an id it does not hold.
- **SystemJS.** The entry is the first named register (SystemJS's named-register extension never
  instantiates an anonymous register beside named ones). Setters must be plain binding, namespace or
  re-export forms, and static cycles are refused; `_context.meta` is refused rather than mapped to
  `import.meta`, whose `url` depends on the file's location. An `execute() {}` method that uses
  `super` at its own level is refused.
- **Bun.** Bun's CommonJS wrapper `(function (exports, require, module, __filename, __dirname) {…})`
  is unwrapped only when the file starts with Bun's `// @bun @bun-cjs` pragma (after a hashbang
  line at most), which is what makes Bun call the function; the file is read as Bun runs it.
  Without the pragma the expression is left as written, and an `async` wrapper is not Bun's.
- **AMD and UMD.** AMD needs every top-level statement to be a `define`, all anonymous (one) or all named;
  dependency cycles, loader plugins (`!`), the `define(function (require) {…})` sugar and a `module`
  dependency are refused. A UMD wrapper's non-factory arguments must be inert (`this`, a literal,
  a `typeof X !== "undefined" ? X : …` guard). The wrapper's branch is chosen from `typeof define`
  tests only: a bare `define` read (`if (define && define.amd)`) throws where `define` is not
  declared, so such a wrapper is refused. The wrapper must be a plain function or arrow
  (not `async`) with distinct parameter names, and a factory or `define` callback that repeats a
  parameter name is refused.
- **Parcel.** Both runtimes are accepted, the registry (`--no-scope-hoist`) only when its runtime
  text matches one of five recorded forms after α-renaming (`parcel/runtimes/`; if a later
  prepare or unminify change rewrites that runtime differently, Parcel registry bundles are refused
  until the forms are re-recorded). Dependency maps with a repeated or `__proto__` key are refused.
  A require that is neither a module of the bundle nor a node builtin is refused: Parcel externals
  are not accepted, which also keeps an `Object.prototype` name (`require("constructor")`, which
  Parcel's runtime resolves through the prototype) from being read as an external. Accepting
  externals later must exclude those names explicitly. The registry call must pass no more
  arguments than its runtime takes; without the fifth (externals) argument the runtime throws on
  any name outside the bundle, so a builtin require is refused there. A builtin require and
  `module.bundle.root(...)` both go through the global `parcelRequire<hash>` slot at run time, so
  a bundle that uses either is refused when a factory can replace that slot: a write to a property
  of that name, the global object (`globalThis`, `global`, `self`, `window`, the factory's fourth
  parameter, `Function`, indirect `eval`) used other than as `G.name`, `G["name"]`, `typeof G` or
  in an equality test, or a write through a computed key on it or on a nested function's `this`.
  An alias of a nested function's `this` is not followed. A lazy chunk that the inputs
  supply is linked into one bundle. A hoisted `var` helper is moved to `_runtime/parcel.js` only
  when no earlier top-level statement names it; otherwise it stays where it is. The scope-hoisted form is accepted only when the statement that
  defines `parcelRequire` equals a recorded text after α-renaming (Parcel 2.16.4, development
  and minified, each as written and as prepare and unminify leave it), and `$parcel$modules` and `$parcel$inits` are each declared once as
  `{}` and named nowhere else at the top level; another Parcel version's definition is refused
  until its text is recorded.
- **Turbopack.** Browser chunks with a registration naming their runtime entries, and server chunks
  given together with Next's page loader and `[turbopack]_runtime.js`, are split. A browser chunk
  holds no runtime, so it is read as if Turbopack's standard runtime loads it, and every such
  unpack prints a note saying so; a chunk the standard runtime would not run that way is refused,
  such as a registration pushed before a module list. A chunk whose
  factories use Turbopack's interop, async or chunk-loading helpers (`i`, `s`, `A`, `l`, …) is
  refused: those helpers live only in the runtime file, so they cannot be moved. Browser multi-chunk
  input is refused (no node oracle).
- **A throwing factory is re-run.** Parcel's and Turbopack's runtimes (like webpack 4's and Metro's,
  as before) keep a module that threw cached; node's `require` deletes the cache entry and re-runs it. esbuild's
  `__commonJS` resets like node and is exact.
- **Several input files are one program.** Unpack never sees the input file
  names, only their texts, so a chunk load (Parcel's `module.require("./lazy.<hash>.js")`, Next's
  loader `R.c(...)`) is linked to the input that defines the ids it names, not to the file the
  specifier names. This is the premise webpack's JSONP chunk merge already used. If the inputs are
  not the whole program, a load that would have read some other file is resolved to the given
  input with the same ids; a load whose ids no input defines refuses the group. Passing input
  names through to unpack, so the specifiers can be checked, is not in 5e. The premise covers only
  chunks a loader loads: Next's page loader must load exactly as many chunks as are given (each
  named once), and a module of a Parcel chunk may be required only by the loader that loads its
  chunk (inside the loader's `.then`); a require of it from anywhere else, which in the bundle
  could run before the chunk exists, refuses the group. A chunk's own modules may require the
  root bundle's modules, which exist before any chunk loads.

### WebAssembly glue

- **An extracted wasm module stays in the code.** With `--wasm-check auto` or `on` and `-o`, a
  byte array, base64 text or `data:` URI that holds a whole module is written to `wasm/`, and the
  literal is left where it is: the program may read it as data (its length, a slice, a hash) or
  hand it to `WebAssembly.Module`, which takes bytes, not a path. The literal is replaced by the
  relative path of the written file (`"./wasm/<name>.wasm"`) only where its one use is a loader
  that also takes a URL: a `data:application/wasm` or `data:application/octet-stream` URI that is
  the first argument of a call of the global `fetch`. That output then fetches the file, so it
  runs where the written file is served beside it at that path; `fetch` resolves a relative URL
  against the page, not the script.
- **Glue names keep the bindings other code can reach.** In a classic script (no `import` or
  `export`) a top-level `var` or function is a property of the global object that later scripts
  read by name, so the wasm glue renamer leaves those names alone, as the main renamer does when
  it keeps script globals. A block-level function declaration in sloppy code and the `var` of the
  same name that it writes (Annex B) also keep their names, and so does such a function when
  code outside its block reads the name. Glue inside a function, in an ES module or in an
  unpacked module file is named as before.
