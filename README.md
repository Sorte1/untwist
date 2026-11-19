<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
    <img src="docs/assets/logo.svg" alt="Untwist" width="270">
  </picture>
</p>

Untwist is a JavaScript deobfuscator, unminifier and unbundler written in Rust on the
[oxc](https://oxc.rs) toolchain. It reads obfuscated, minified or bundled JavaScript and gives back
readable source: string arrays decoded, control flow unflattened, minifier idioms undone,
transpiler helpers turned back into native syntax, bundles split into modules and, on request,
identifiers renamed.

It is a reimplementation of [webcrack](https://github.com/j4k0xb/webcrack) with two goals: speed,
and deobfuscating hostile input without running it. Untwist never executes the program it is
given. String decoders run inside a sandboxed, step-budgeted interpreter, and a decode is applied
only where Untwist can show from the program text that it gives the same result the original
would.

Try it in the browser at **[untwist.dev](https://untwist.dev)**. The playground runs the same
engine as WebAssembly, and your code never leaves the page.

## What it does

- **Deobfuscate.** Finds javascript-obfuscator / obfuscator.io string arrays with their rotators,
  decoders (base64, RC4) and wrapper functions, and replaces each decoder call with the string it
  returns. It also removes the leftover machinery, inlines control-flow proxy objects, unflattens
  `switch` dispatch loops, strips self-defending and debug-protection code, and folds constant
  expressions.
- **Unminify.** Undoes minifier idioms: `!0` becomes `true`, comma sequences become statements,
  ternaries become `if`/`else`, Yoda conditions are flipped, `void 0` becomes `undefined`, and so on.
- **Untranspile.** Turns TypeScript, Babel, SWC and esbuild helpers for spread, destructuring,
  classes, generators and `async` back into native syntax.
- **Unpack.** Detects webpack 4/5 (including JSONP chunks), browserify, esbuild and Bun, rolldown
  (also Vite 8), SystemJS, AMD, UMD, Parcel, Metro, Turbopack and Closure bundles, extracts their modules and rewrites
  `require`/`import` specifiers to relative paths. Rollup and Vite scope-hoisted bundles can be
  split on request. Several input files can be unpacked together. The unpacked tree is untrusted
  code: its file names and contents come from the bundle. Do not open it with an editor or tool
  that runs project configuration on its own.
- **Rename.** Optional: `mangle` (short names, compatible with webcrack's `-m`), `simple` (short names per function),
  `unique` (readable, program-wide names such as `fn1`, `arr2`, `handler3`) or `smart` (names from evidence: `i`/`j`
  for loop counters, `item`/`index` and `acc` for array callbacks, `err` in `catch`, `fs` for `require("fs")`,
  `canvas` for `document.createElement("canvas")`, `ctx`, `data`, destructured keys and the property a function is
  exported or stored under; also the property a value is read from or written to, constructors (`socket` for
  `new WebSocket`, `vector3`, `floats`), element ids and storage keys (`chatList` for `getElementById("chatList")`),
  event handlers and their events (`onKeydown(keyEvent)`), timer and frame callbacks, `fetch` responses and bodies,
  setter values, export aliases, `is*` names for functions that return booleans, PascalCase for functions only used
  with `new`, and a parameter's name when every call passes the same named value; a name is reused in sibling scopes
  and numbered only where it would clash, and human-written names are left alone; long hex tokens such as
  `b475796ed633d5fd0485` count as machine-made here and are renamed when evidence names them).
- **CommonJS wrappers.** In every mode, the lazy `requireX()` wrappers that `@rollup/plugin-commonjs` writes
  (`if (hasRequiredX) return x; hasRequiredX = 1; ...`), terser's inlined forms of them, rolldown's
  `__commonJSMin` factories and the plugin's interop helpers (`commonjsGlobal`, `getDefaultExportFromCjs`,
  `getAugmentedNamespace`, `commonjsRequire`) get their names back when minification or obfuscation took them:
  `requireModule3`, `hasRequiredModule3`, `module3`, or the module's own name when one survives. Only names change;
  every wrapper still runs on its first call. Without `--rename` every function keeps its name, the wrapper functions
  and interop helpers included; only the flag, holder and parameter bindings are renamed (rename.md, "CommonJS
  wrapper names"). `--disable name-commonjs-wrappers` turns it off.
- **Libraries.** Recognises core-js 3 (`internals/*` and `modules/*`, every minor 3.0 to 3.51),
  regenerator-runtime, tslib, Babel helpers (`@babel/runtime` and inlined) and `@swc/helpers` by the exact shape
  of their functions, minified, mangled or obfuscated, and puts `// core-js 3.38–3.51: internals/global-this.js`
  above each matched module. With any `--rename` mode the matched bindings get the library's own names
  (`requireGetBuiltIn`, `getBuiltIn`, `isCallable`, ...); the report lists each library with its version range.
  `--disable library-names,library-rename` turns this off.
- **JSX.** Rebuilds JSX from `React.createElement` and the automatic `jsx`/`jsxs` runtime.

## Install

### CLI

```sh
cargo install untwist-cli
```

This installs the `untwist` binary. It needs Rust 1.98 or newer.

### Rust library

```sh
cargo add untwist
```

```rust
use untwist::{Options, Pipeline};

let output = Pipeline::new(Options::default()).run("var a = !0;");
println!("{}", output.code());
output.write_to("out-dir")?;
```

### npm (WebAssembly)

```sh
npm install untwist
```

```js
import init, { run } from 'untwist';

await init();
const result = run(source, '--unsafe', false);
console.log(result.stdout);
```

The package is the single-threaded WebAssembly build the playground uses; see its
[README](crates/untwist-wasm/npm/README.md) for the full API and the flag format.

## Usage

```
untwist input.js                 # print the result to stdout
untwist input.js -o out-dir      # write deobfuscated.js (and modules, if a bundle is found)
untwist a.js b.js -o out-dir     # unpack several files together
cat input.js | untwist           # read from stdin
```

Common flags:

| Flag | What it does |
| --- | --- |
| `-o, --output DIR` | Write `deobfuscated.js`, and, when a bundle is found, `bundle.json` and one file per module. A `.cjs` or `.mjs` input keeps its extension (`deobfuscated.cjs`, `deobfuscated.mjs`), so node reads the file under the input's goal. With several inputs each is written as `deobfuscated.<name>.js`; when only some of them form the bundle (a chunk unpacked without its loader, or an entry whose chunks are not bundles), every input is written under its own relative name instead, so the tree runs as the inputs did, and no module takes one of those names. |
| `-f, --force` | Overwrite the files in `DIR` that this run writes and remove the ones a previous run left there. Input files are never overwritten. |
| `--unsafe` | Use the unsafe lane (see below). |
| `--modern BASELINE` | Opt-in: remove core-js polyfills of features that are native in `BASELINE` (see [Modern mode](#modern-mode)). |
| `--types[=jsdoc\|ts]` | Experimental, opt-in: annotate only types that hold for every value on every run, as JSDoc (the default) or TypeScript (see [Proven types](#proven-types)). |
| `--rename MODE` | `none` (default), `mangle`, `simple`, `smart` or `unique`. `-m` is short for `--rename mangle`. |
| `--no-deobfuscate`, `--no-unminify`, `--no-unpack`, `--no-jsx` | Skip a stage. |
| `--split-hoisted` | Also split Rollup/Vite scope-hoisted bundles, when the split provably keeps behaviour. |
| `--split-assume-modules` | With `--split-hoisted`: assume the split tree is ES modules for reading and for running in node, so `document.currentScript`, classic-script load timing and sloppy-mode top-level semantics are not preserved. Lifts the `currentScript` refusal, splits a bundle that sits inside an outer wrapper function, and makes every Rollup commonjs wrapper its own module while everything else stays in the entry. See [known divergences](docs/known-divergences/unpack.md). |
| `--single-file` | Write every unpacked module into one file, with a banner per module. |
| `--source-map [file\|inline]` | Emit a source map from the output back to the input. `file` (default, needs `-o`) writes `<file>.map` next to every written file with a `sourceMappingURL` comment; `inline` embeds the map as a data URL. Unpacked modules get one map each. The code itself is the same with or without the flag. |
| `--input-source-map PATH\|inline` | With `--source-map`: the input already has a source map (a file, or its inline data URL); map through it to the original sources. A source that is absolute, has a scheme or climbs out with `..` is written as `input-map:///<name>`; the input map's `sourceRoot` is joined into each source first and not copied. |
| `--wasm-check off\|on\|auto` | Look for wasm modules embedded in the output. `off` (default) only prints a note when the output may hold one; `auto` finds a byte array or base64 text that holds a whole module with imports or exports, writes it to `wasm/` and names its glue (feature-detect probes are left alone); `on` reparses every output to look, which can be slow on large files. |
| `--wasm-dir DIR`, `--wasm NAME=PATH` | Supply wasm modules the input loads from elsewhere, so their import and export names can name the glue. This works with every `--wasm-check` mode. |
| `--fetch-wasm`, `--wasm-base URL` | Download the wasm modules nobody supplied. Nothing is fetched without this flag. The URLs come from the input, so each one is printed before it is requested, and one run fetches at most 16 modules, 64 MiB and for 120 seconds. Redirects are followed three times at most. A host that resolves to a loopback, link-local or private address is refused, and no proxy from the environment is used. |
| `--fetch-wasm-local` | With `--fetch-wasm`, also fetch from loopback, link-local and private addresses, for input that loads its wasm from your own machine or network. |
| `--disable PASS` | Turn off individual passes; `--list-passes` prints them all. |
| `--report text\|json` | Print what every stage and pass did. `-v` is short for `--report text`. |
| `-j, --jobs N` | Process up to `N` inputs in parallel. The output does not depend on `N`. |
| `--max-memory SIZE` | Memory the inputs being processed may use (the texts the run holds are not counted), such as `512MiB`, `8G` or `unlimited` (default `4GiB`). At three quarters of it an input stops its passes and keeps the output so far, with the note `budget-memory`; at the full size that input is stopped and passed through unchanged, whatever its passes are doing. `--report` shows the budget and the peak. |
| `--timeout SECS` | Wall-clock seconds processing the inputs may take (default: no limit); writing the output and `--fetch-wasm` are not limited by it. At the limit every input stops its passes and keeps the output so far, with the note `budget-time`; an input still running a while later is passed through unchanged. |
| `--fuel N` | Interpreter steps a string-array decode may spend before giving up (default 50,000,000 plus 500,000 per distinct decoder call, at most 1,000,000,000; 1,000,000,000 with `--unsafe`). All decodes of one input share a pool of 32 times that per-decode figure or 2,000 steps per input byte, whichever is larger; once it is spent the remaining decodes are left encoded with a note. |

When an input stops its passes at a budget and keeps the output so far, that output still runs like the input: every rewrite in it was proven on its own.

`untwist --help` lists every flag.

### WebAssembly

untwist never runs or rewrites a wasm module; it reads only its import and export names (and its
name section, when it has one) to name the JS glue around it. Each name is taken only after a
check that every import key and export read in the glue exists in the module; a module that does
not match gives a warning and names nothing. Embedded modules are found with `--wasm-check`, and
modules loaded from elsewhere are supplied with `--wasm-dir`, `--wasm` or `--fetch-wasm`. An
embedded module is written to `wasm/` and its literal stays in the code; only a `data:` URI passed
straight to `fetch(...)` is replaced by the path of the written file. In a classic script the
glue's top-level `var` and function names keep their names, because other scripts can reach
them, and so do a sloppy-mode block function and the `var` of the same name that it writes (or
code outside the block that reads the function's name). In a
webpack bundle the names reach the module files that import from or export to the wasm module,
and the same names are applied to the combined `deobfuscated.js`. That includes wasm-bindgen
glue (bundler target) in webpack 4 and 5 bundles that reaches the module through
`import * as wasm`, directly or handed over with `__wbg_set_wasm`: a wrapper that calls one
export, which no other function calls, takes that export's name. Glue and module given as
separate input files are not linked. For Emscripten glue whose
module has no name section, exports are named from `Module._name` wrappers and the export kind
(`wasmMemory`, `wasmTable`). Minified import names like `a.a` say nothing about the function
behind them, so an import is named only when its function has the shape of an Emscripten library
function (`HEAPU8.copyWithin(dest, src, src + num)` is `_emscripten_memcpy_big`) and the module's
import has that function's signature. Such a module must also have every import the glue
provides and no import the glue leaves out; otherwise nothing is named.

Exit codes: `0` success; `1` an error, a parse error (output is still produced) or a panic in a
pass; `2` a file this run would write already exists in the output directory (use `-f`); `3` an
input was too deep to process safely and was passed through unchanged.

On Unix the pipeline runs in a worker process, so an input that crashes it (a stack overflow or a
failed allocation) is passed through and the rest of the run is kept. Windows has no worker: the
memory and time limits still stop an input and pass it through, but a crash ends the whole run
without output.

## Safe and unsafe lanes

By default Untwist runs in the **safe lane**. A rewrite is made only when it can be proven from
the program text. A string-array call is decoded only if nothing in the program can change what
the decoder returns at that point: no patched built-in it relies on, no alias that reaches the
array or decoder before the rotator runs, no code that is built at run time from strings. When
the proof fails, the call stays in place and the report says why. The output behaves like the
input.

The safe lane rests on a few stated premises about code it cannot see. One input file is a closed
program. Several classic scripts given in one run share one global scope, as scripts on one page
do: a top-level `var`, function, `let`, `const` or `class` of one is treated as read and written by
the others when another input names it (a free reference or a top-level declaration of its own) or
reaches globals through a computed key or code text, and is then not inlined away, removed or
renamed, in both lanes; a string array, decoder, decoder alias or protection named that way is kept
and nothing is decoded through it. A built-in name (`eval`, `String`, `atob`, ...) that another input declares at its top level, assigns as a bare name, sets as a property of the global object (`this.X =`, `globalThis.X =`, `window["X"] =`, a definer such as `Object.defineProperty(globalThis, ...)`), or declares or assigns in a code text it runs (`eval`, `(0, eval)`, `Function`, a timer string) is not the built-in for any input, so no fold relies on it. A global-object write with a computed key, or a code text that is not a literal, counts for every name. In the default lane an input whose own built-in scan is not clean (it uses a built-in, the global object or a code runner outside the listed harmless shapes, or patches a built-in member) counts for every name of every input, and no input folds a built-in call (`" x ".trim()`, `["a", "b"].join("")`, `String.fromCharCode(...)`) unless the built-in scan of every input is clean. In the default lane a top-level binding of one input that another input names (assigns, redeclares, reads, or uses as a key) is not read from its initialiser when the first input is judged, so a key, holder, member name, field or element another input may rewrite counts as unknown; a call on a value that comes from another input's function is judged as a possible code runner; and `new F()` is a fresh object only when `F` is used only as a callee and its `prototype` is never assigned anything but a plain object literal. A refusal for this cause names the input, and the line when one statement is the cause. The default lane also assumes no input hands code text to a DOM API that evaluates it (`document.write`, a `javascript:` URL, a script element's text, an event-handler attribute string). Renaming never renames a classic script's top-level bindings in the safe lane. Under `--unsafe`,
when two or more classic scripts are given or another input may observe the global object's
names, they keep their names too (`--rename unique`, whose counter is shared by every input,
still renames them when only other scripts are the reason), and no input's new top-level name is a
name another input binds at the top level, reads free or uses as a key. Inputs that share no names
come out as they would alone. A module (a file with `import`, `export`,
`import.meta` or top-level `await`) is closed apart from its exports. Code from outside the file
(`require` results, host functions, scripts not given to Untwist) writes no built-in. It also does not depend on
the names the obfuscator picked for the string array and its decoders, because that code was
written before those names existed. So when such code is handed a value that may hold the global
object, the machinery can still be removed. Outside code that prints that value still counts as
observing it (`require("util").inspect(globalThis)` keeps the machinery). A removal that relies
on this premise says so in the report (`the machinery was dropped under the outside-code
premise`). Nor does any code give `Object.prototype` an accessor for a key the obfuscator
generated, so storing an object under such a key runs no code. Nor does any code give
`Object.prototype` a property, data or accessor, named like a module id, a chunk id or a require
request of a bundle whose runtime looks modules or chunks up in a plain object (webpack 4 and 5
and their chunks, browserify, Parcel, Turbopack): such a runtime would find that property in its
`{}` cache, module map or installed-chunk table instead of the module or chunk, while the written
files are loaded by node; unpacking does not check for such a write. Nor does any code replace `Function.prototype.call`, `apply` or `bind`:
those runtimes start their modules through `.call` (webpack 5 only where a module uses `this` as its
exports), which node's loader does not use for a written file. Nor does any code reach node's
module objects or loader through `process` taken as a value (`var p = process; p.mainModule`,
`globalThis.process.mainModule`): a module that names `process.mainModule`, `process.getBuiltinModule`, a
computed member of `process`, or requests the `module` or `process` built-in is not split, but an
alias is not followed, since most bundles pass `process` around. And
no value the program tests is `document.all`, so `x != null ? x : y` becomes `x ?? y`: the two
differ only for that object, which compares loosely equal to `null` without being nullish. Nor does any code read a function's caller through `.caller`, `arguments.callee.caller` or a stack frame (`Error.prepareStackTrace` call sites with `getFunction()` or `getFunctionName()`) to observe which function called it (the non-standard `Function.prototype.caller`; strict code cannot): inlining a control-flow object's call proxy removes the proxy's frame, so such a read would see the function that held the call instead of the proxy. The default lane also assumes obfuscator.io's self-defending check would pass on the input: removing the trap may make an input that would trap (for example one reformatted after obfuscation) run normally. It does not assume an obfuscator.io domain lock would pass: the lock reads the host's `document` and `location` and enumerates the global object's names, so the default lane keeps it, the protections registered with it and the machinery it can see, and says so (`a domain lock was found …`); `--unsafe` assumes the lock passes and removes it. The default lane keeps a protection that may still run, one it cannot read or prove inert (a self-defending callback is inert only when the built-in scan proves that nothing in the program can replace a built-in member the callback calls), and when that is a block registered through a call controller or a debug-protection timer it keeps every block registered that way and every debug-protection timer with it, including another layer's console silencing, because removing one layer's silencing and keeping another's changes what the program prints, and removing the silencing and keeping a timer, or the reverse, changes what it prints or whether it keeps running. Untwist also assumes no debugger or inspector session changes the program's values while it
runs; every lane and every proof rests on this.
[`docs/known-divergences/deobfuscate.md`](docs/known-divergences/deobfuscate.md) lists every
premise.

The safe lane never changes the `name` of a function or class unless you ask for renaming. Built-ins
and the host read `name` with no key in the program (`Error.prototype.toString`, WebCrypto
algorithm objects, DOM sinks), so whether a program reads names cannot be decided from its text:
every pass and every unpack rewrite keeps each function's and class's `name`, writing
`(0, function () {})`, `{ old: fn }.old` or `let renamed = class old {}` where a value would
otherwise gain or lose one, and leaving the code as it was (or the bundle whole) where no such form
exists. With `--rename` (any mode), a function's or class's name, and anything derived from it
(`Error.prototype.toString`, host APIs reading `name`), may change: renaming the binding that gives
the name is what was asked for. The renamer still keeps, as a best effort, the name of a function or
class when it sees the file read `name` (`.name`, a computed key that is not a number, a numeric
loop counter or a literal naming another property, reflection, copies of own properties, code text),
but that is not a promise. The other passes keep names under `--rename` as they do without it, so a
function a pass moves keeps the name it had.
[`docs/known-divergences/rename.md`](docs/known-divergences/rename.md) has the details.

The **unsafe lane** (`--unsafe`) makes the assumptions webcrack makes: built-in objects are not
patched, even when the program visibly patches one, a direct `eval` reads and writes no local
variable (the lexical passes still refuse a program whose `eval` may), a built-in prototype that the
program reaches through a value it reads (`[].__proto__`, `o[key]`, `Reflect.getPrototypeOf(x)`)
is not patched through that value, no code observes a classic script's top-level `var` or
function through the global object or by its name other than in the ways the scan sees (so
renaming may rename them; the safe lane never does). The scan sees property reads off the global
object and code built from text: a literal text that names a top-level binding, a text that is
not a literal, or a text that may hand back a code runner (`Function("return eval")()(s)`,
`Function("return Reflect.set")()`, a `constructor`, a timer) keeps every top-level binding's name, `let`, `const` and `class`
included, as does a read of the global object by a key the scan cannot resolve, and no new
top-level name is a word that a string of the file spells, also when the string is joined
through a constant binding (`Function("return typeof a")`). A value read by a key the scan
cannot resolve is not taken to be a code runner, so calls made with it are not judged as code
text (a runner read by name or by a key the scan resolves still is). No code observes or
replaces the obfuscator's machinery, protections or decoder aliases through the global object,
reflection or global code text (so a top-level string array, decoder, decoder alias,
self-defending or debug-protection binding of a classic script is removed, and calls through it
are decoded, even when the program enumerates or tests the global object's names, reads or writes
them by key, through `Object.assign`, `Reflect.set`, `Object.defineProperty` or a loop over keys,
or runs code text through any runner, `(0, eval)`,
`Function`, a timer string, or a runner that code text hands back such as
`Function("return eval")()(s)`). No function declared in a block or directly in an `if` clause
overwrites a decoder alias, a decoder wrapper or a constant used as a decoder argument through
Annex B hoisting, and no direct `eval` redeclares one in its own scope (`var d = dec;
{ function d() {} } d(1)`, `var d = dec; if (1) function d() {} d(1)` and
`var d = dec; eval("function d() {}"); d(1)` are each decoded as a call of `dec`). A few passes that rest on
heuristics are allowed to run. These are `template-literals` and JSX reconstruction (`jsx`, `jsx-new`).
`dead-code`, which deletes branches behind constant comparisons, runs in both lanes.
`fold-pure-calls` also folds calls to a pure helper that is passed around as a value or that the
program could rebind through a key it cannot resolve.
A Metro bundle whose file does not hold Metro's own `__d`/`__r` runtime is unpacked on the trust
that those names are Metro's.
A function's caller (`.caller`, `arguments.callee.caller`, a stack frame's `getFunction()`) is
the bundle's own function in the bundle and node's module wrapper in a written file. The safe lane
splits a bundle only when no code can see that frame: every function in the bundle's input is
strict code and every written module file is strict (an ES module, or a file that starts with
`"use strict"`), because V8 gives `null` for the caller of a function called from strict code and
`undefined` for `getFunction()` on strict frames. Bundles with sloppy code, which covers most
webpack 4 and CommonJS bundles, are kept whole in the safe lane. The unsafe lane assumes no read
of a function's caller or arguments reaches a bundled module's function and splits them anyway.
The unsafe lane also splits a bundle whose modules import files given beside it, on the premise
that the inputs given beside a bundle are the files its relative imports load and that the bundle's
input is not imported back by name; the safe lane keeps any bundle with a relative `import()`,
`import` or `export … from` whole. It assumes no code text (`Function`, indirect `eval`, a timer
string) reaches `import()`, node's module objects or loader, and that the program does not read its
own file name or the main module's path (`import.meta.url`, `__filename`, `require.main`,
`process.argv`); the safe lane refuses those. Both lanes assume no code reads file names out of a stack trace (`new Error().stack`).
An input is taken as an ES module, as node 22 takes it, when it is named `.mjs`, when its own text
(before any pass runs) does not parse as a script but parses as a module (`import`/`export`,
`import.meta` or a top-level `await x`, also in code a later pass removes), or when it declares
`require`, `exports`, `module`, `__filename` or `__dirname` with a top-level `let`, `const` or `class`,
which node's CommonJS wrapper does not allow. A `.cjs` input is always a script, as in node. Any
other `.js` input is a script or CommonJS, also one that parses only by calling a global named
`await` (`await (x)`). Untwist decides between script and module from the file extension and the
text only: it does not read the `"type"` field of the nearest `package.json`, so a `.js` file under
`"type": "module"` (or a `.js` file with module syntax under `"type": "commonjs"`) is processed as
node would process it with no `package.json`; name such a file `.mjs` or `.cjs`. An input that
node 22 rejects with a SyntaxError is passed through unchanged with an error: an early error (a
legacy octal literal in strict code, an invalid regular expression pattern, a parenthesised
destructuring target), syntax node 22 does not read (`using`,
decorators, `accessor`, `import defer`/`source`, `assert` import attributes, regular expression
modifiers and repeated group names), and `<!--` in module code. Only the checks that a corpus of
probes run against node 22 shows node also makes refuse an input, so for example a legacy octal
literal directly in a sloppy file's class body, which V8 accepts, is read. An input that node runs
but untwist's parser cannot read (a call as an assignment target, `for (yield of ...)`) is passed
through unchanged with an error that says so, and a function declared directly in an `if` beside a
same-named `let`, `const` or `class` is passed through unchanged because untwist's binder does not
model its scope. The module code of a
bundle in an ES module input is strict, so its written module files get `"use strict"` (or are ES
modules themselves), and when the passes remove the only module syntax of such an input, its
output ends with `export {};` so that node still runs it as a module.
This decodes more on aggressively protected code. It is the right choice when you trust that the
input does not tamper with the runtime, and the wrong one for code written to fool analysis.

Every deliberate difference from webcrack is listed, with the reason, in
[`docs/known-divergences.md`](docs/known-divergences.md), where webcrack is called the reference
deobfuscator.

## Modern mode

`--modern BASELINE` removes core-js polyfills when the code will only run on engines that have
the feature natively. It is off unless you ask for it, and `--unsafe` does not turn it on.
`BASELINE` is an ES year (`es2017` to `es2024`), engines and versions (`chrome 120, firefox 121,
safari 17, node 20`), or both; an engine named twice counts with its lower version.

| Year | Engines it stands for |
| --- | --- |
| `es2017` | chrome 57, edge 15, firefox 52, safari 11, ios 11, node 8.0.0, opera 44, samsung 7.0 |
| `es2018` | chrome 64, edge 79, firefox 78, safari 16.4, ios 16.4, node 10.0.0, opera 51, samsung 9.0 |
| `es2019` | chrome 73, edge 79, firefox 78, safari 16.4, ios 16.4, node 12.0.0, opera 60, samsung 11.0 |
| `es2020` | chrome 80, edge 80, firefox 80, safari 16.4, ios 16.4, node 14.0.0, opera 67, samsung 13.0 |
| `es2021` | chrome 85, edge 85, firefox 80, safari 16.4, ios 16.4, node 15.0.0, opera 71, samsung 14.0 |
| `es2022` | chrome 94, edge 94, firefox 93, safari 16.4, ios 16.4, node 16.11.0, opera 80, samsung 17.0 |
| `es2023` | chrome 110, edge 110, firefox 115, safari 16.4, ios 16.4, node 20.0.0, opera 96, samsung 21.0 |
| `es2024` | chrome 119, edge 119, firefox 145, safari 17.4, ios 17.4, node 22.0.0, opera 105, samsung 25.0 |

A year stands for the first engine versions that shipped all of its syntax and built-ins, even
with known bugs (from `@mdn/browser-compat-data`); core-js-compat then decides per engine
whether core-js still replaces a buggy native there. The table is generated by
`cargo xtask modern-table` from the versions pinned in `tools/core-js-compat`.

Only modules that the library fingerprints recognise exactly as core-js are touched. A
polyfill module (`modules/es.*`, `modules/web.*`) is removed when core-js-compat, for the
recognised core-js version, says no engine in the baseline needs it, which also covers the
engines where core-js replaces a buggy native. Proposals (`esnext.*`), modules core-js still
replaces in one of the engines, modules whose exports other code reads, and modules that call
code not recognised as core-js are kept. A module that runs a module that is kept is kept as
well, so a polyfill the baseline needs never loses the call that ran it. Helper modules
(`internals/*`) that nothing uses any more are removed after them. Only core-js code is
removed: the module wrappers, their own variables and the statements that run a removed
module; code around them is never touched. core-js-pure (whose `path` object holds the
polyfills instead of the global object) is detected and left as it is.

In the core-js code that stays, calls of exactly matched helpers become the native form where
it behaves the same in every baseline engine: `wellKnownSymbol("iterator")` becomes
`Symbol.iterator`, `getBuiltIn("Reflect", "ownKeys")` becomes `Reflect.ownKeys`, `hasOwn(it, key)`
becomes `Object.hasOwn(it, key)`, an uncurried method `stringSlice(s, 1)` becomes
`String.prototype.slice.call(s, 1)` (only when core-js does not replace that method in the
baseline), and `isCallable(it)` becomes `typeof it == "function"` only when no browser is in the
baseline (in browsers core-js also accepts `document.all`). `requireObjectCoercible` is left as
it is, since no native form throws the same error. A call is rewritten only where it is used as
a value (not under `delete`), where the built-in's name is not shadowed, and where the helper's
variable is certainly set before the call runs.

The rewritten calls read the engine's built-ins when they run, where core-js read them once when
it loaded (`hasOwn` keeps the `Object.hasOwn` it saw then, an uncurried method keeps the method).
So the output assumes that the program does not replace built-ins after core-js loaded. Removing
a module can also move the first run of a helper module it used to run, which matters only to
code that changes built-ins between the two points.

The report lists every removed and kept module with the reason and every rewritten call
kind, and says that the output assumes such an engine with its own built-ins; the output starts with
a comment saying the same. These are informational notes: one summary line counts the removed and
kept modules (the kept ones by reason), and the playground shows the per-module list in Details,
grouped by reason, instead of in Diagnostics.

Modern mode is part of the CLI, the Rust library (cargo feature `modern`, on by default), the
WebAssembly build and the playground (Settings, Modern baseline), where the core-js table is
fetched only when the input holds core-js code.

Today this covers Rollup bundles built with `@rollup/plugin-commonjs` and `strictRequires`
(the lazy `requireX()` wrappers and the inline wrappers Rollup makes for modules required
once), in one program (with `--split-hoisted` every wrapper becomes its own module and they are
left as they are). Scope-hoisted Rollup output, webpack module maps, esbuild `__commonJS`
bundles and wrappers a minifier inlined into their caller are recognised but left as they are.

## Proven types

`--types` (or `--types=jsdoc`) keeps JavaScript and adds `/** @type */`, `@param` and `@returns`
comments; `--types=ts` writes the output as TypeScript (`.ts`, or `.tsx` when JSX is left). It is
experimental and off unless you ask for it. An annotation is written only when untwist proves
that every value the position can hold, on every run, has that type; everything else stays as it
was. `number` includes `NaN` and `-0`, a literal type is never `NaN` or `-0`, and no annotation is
ever `any` or `unknown`.

What it proves today is only what needs no premise about the rest of the realm: literals and the
closed set of values a local binding is written with, the results of operators, function returns
(an `async` function's as `Promise<…>`), and the parameters of a strict function (module code,
class code or code under `"use strict"`) whose every call is visible. `.caller`, `.arguments` and
the stack-trace API never hand out a strict function, so no code outside the run can call one with
other values. A function is never treated that way when it is exported, reads `arguments`, is
called with a spread, or is reached any other way, and no function at all when any input or module
of the run can read a function's frame (`.caller`, `.arguments`, stack-trace functions, a `with`
statement, or code text that could). A sloppy function's parameters stay untyped, and so do the
results of built-ins such as `Math.floor`, `parseInt` or `"".slice`: both would rest on proving
that no code in the realm patches a built-in or reaches a frame, which this version does not try.
A direct `eval` (also through a local binding named `eval`) leaves untyped every binding it can
reach, and in sloppy code it also turns off the types that rest on global names it could shadow
(`undefined`, `NaN`, `Infinity`); a constant-string `eval` that only reads leaves out only the
names it reads. A `with` statement leaves the file untyped, and so does an object literal with a member whose
computed key is `Symbol.<name>` (tsc 7.0.2 reports the type errors around such an object only in
part, and differently from run to run, until a type is written); an operator result is typed only
when every operand is a proven primitive. Script-level `var`s, exported
bindings, objects, arrays, classes and the DOM get no types.

Every proven type is written, also when TypeScript would infer it anyway, unless writing it would
give the TypeScript checker a new error (for example a literal type compared with a value outside
it, which the checker narrows away), so the annotated file checks no worse than the untyped
output under `strict` with `noImplicitAny` off. `--report` counts the annotated positions, how
many of them TypeScript infers anyway, and the refused ones. The flag cannot be combined with
`--source-map`.

A file is written as `.ts` only when its TypeScript parse, stripped of the annotations, is the same
program as its JavaScript parse (`a < b > (c)` is a comparison in JavaScript and a generic call in
TypeScript) and it uses no name node's type stripping rejects (`await`, `yield` as identifiers);
otherwise it keeps its `.js` name and is written untyped. CommonJS files (free `require`,
`module` or `exports`) keep `.js` under `--types=ts` and stay untyped; `--types` (JSDoc) types them.
When split ES modules become `.ts`, the `import`, `export … from`, `import()` and `require`
specifiers that name them are rewritten to the new names, so the tree runs under node's type
stripping and checks with `allowImportingTsExtensions`. When a module loads another by a computed
name, or uses `require` other than as a call with one literal, or `import.meta`, every file keeps
its `.js` name and its untyped text. With `--single-file`, the joined file is TypeScript only when
every part is; otherwise every part is written untyped.

The types run after every other stage, on the final text, so they describe the program untwist
prints. The checks behind this live in `crates/untwist-types/tests` (one row per hazard), in the
node run of the fixtures with every annotation turned into a runtime check, and in
`tools/types-check`, which runs the pinned `tsc` over every annotated fixture output against the
same text without annotations and fails on any new diagnostic. It runs `tsc` three times on each
side (`TYPES_CHECK_RUNS` sets the count) and compares each annotated run with its own plain run,
so a new diagnostic in any one run fails the check, and more runs can only find more.

Known limitation: tsc 7.0.2 may report fewer diagnostics for unannotated code whose types carry
well-known-symbol keys (`[Symbol.iterator]`, `[Symbol.toPrimitive]`, ...), and which ones it drops
changes from run to run. A typed file can then show diagnostics that the plain file's run left out.
The annotations themselves are still true. A file with an object literal keyed by a literal
`Symbol.<name>` is left untyped to make this rarer, but an aliased `Symbol` or a copy of such an
object through a spread is not caught. See
[`docs/known-divergences/types.md`](docs/known-divergences/types.md). `--against-js` also compares each
`.ts` output with the untyped output as `.js` under `checkJs`; the diagnostics it reports come from
renaming JavaScript to `.ts` (in a `.ts` file every parameter is required, an object literal gets a
fixed shape, and so on), not from the annotations, so they do not touch the promise that every
written type holds. JSDoc mode keeps `.js` and has none of them, which is why a bare `--types` and
the playground's first choice are JSDoc.

## Playground

[untwist.dev](https://untwist.dev) runs Untwist in a Web Worker. Paste, drop or open a script and
press Run. You can browse and download the unpacked modules, compare input and output side by side,
read diagnostics and the run report, and edit the output with go to definition, find references
and rename. The output is byte for byte what the CLI gives for the same flags. The source is in
[`web/`](web/README.md).

## Compared with webcrack

| | Untwist | webcrack |
| --- | --- | --- |
| Language | Rust (oxc) | TypeScript (Babel) |
| Runs the input's code | Never; decoders run in a sandboxed, budgeted interpreter | Runs decoders in `isolated-vm` |
| Decoding under tampering | Refuses what it cannot prove, by default | Assumes built-ins are unpatched |
| Bundlers | webpack, browserify, esbuild/Bun, rolldown/Vite 8, SystemJS, AMD, UMD, Parcel, Metro, Turbopack, Closure, Rollup/Vite (opt-in) | webpack, browserify |
| Untranspile helpers | TypeScript, Babel, SWC, esbuild | No |
| Rename modes | `mangle`, `simple`, `unique` | `mangle` |

Untwist's output is checked against webcrack's with differential harnesses (`cargo xtask
diff-unminify`, `diff-deobfuscate`, `diff-unpack`, `diff-mangle`, `diff-jsx`), which run Untwist in
an internal comparison mode limited to the passes webcrack has. Decoded output is also checked by
running both the input and the output under Node and comparing what they print. `cargo xtask
obf-options` runs javascript-obfuscator 5.9.1 output for every free option, pairs of the
high-impact ones and the four presets through both lanes and a pinned webcrack, and requires each
output to keep no obfuscator machinery and to print what the original program prints.
`--suggest-expect <file>` writes a whole replacement for its `EXPECT`. Each failure is named by its
cause, and each cause keeps the reason it already has in `EXPECT`.
`cargo xtask obf-options --dir fixtures/obfuscator-versions` runs the same checks on the version
matrix that `tools/obfuscated/versions.mjs` generates: twenty javascript-obfuscator releases from
0.10.2 to 5.9.1, each with its own string array options (encoding, rotate, shuffle, index shift,
wrappers, calls transform, threshold) and one extra transform, placed at the top level, in an IIFE,
in a webpack 4 module or around a whole webpack 4 bundle, then left as is or minified with
uglify-js or terser. The cells cover every version × encoding × container × minifier and every
pair of the other choices. Its `EXPECT` says for each cell whether the run is refused with a named
reason or `MISSED` (no string array found, or calls left with no note).
`cargo xtask obf-fuzz --seed <n> --count <n>` draws random programs from a grammar of those
features, obfuscates them with random option sets and rewires the decoder (aliases, wrappers, inert
statements, strict mode, a wrapping function), then judges both lanes the same way. A failure is
grouped by the cause key of its first blocking note and shrunk to a small repro. Only causes listed in
`fixtures/fuzz/EXPECT` (open gaps) or `fixtures/obfuscator-io/SOUND_REFUSALS` (refusals that are the
right answer, on programs with the named feature) are allowed.

## Building from source

```sh
cargo build --release -p untwist-cli    # target/release/untwist
cargo test --workspace
```

The workspace layout:

| Crate | Contents |
| --- | --- |
| `untwist` | The pipeline and the library API |
| `untwist-cli` | The `untwist` command-line tool |
| `untwist-core` | Parsing, printing and the pass framework |
| `untwist-eval` | Constant folding and the sandboxed interpreter |
| `untwist-deobfuscate` | String arrays, control flow and protections |
| `untwist-unminify` | Unminify and untranspile passes |
| `untwist-unpack` | Bundle detection and module extraction |
| `untwist-rename` | Identifier renaming |
| `untwist-jsx` | JSX reconstruction |
| `untwist-wasm` | The WebAssembly build (npm `untwist`) |
| `untwist-regress` | A fork of [regress](https://github.com/ridiculousfish/regress) with a step-budgeted search |

Some tests read a large real-world sample at `fixtures/real-world/game.js`, which is not
distributed with the repository; they are skipped when it is absent. The `cargo xtask diff-*`
harnesses need a built webcrack clone in `reference/` (or set `UNTWIST_REFERENCE_DIR`) and a CLI
built with `cargo build --release -p untwist-cli --features test-hooks`; `cargo xtask obf-options
--webcrack` also needs `npm ci --prefix tools/webcrack`. Test inputs are described
in [`fixtures/README.md`](fixtures/README.md).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at
your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.

Untwist's behaviour follows webcrack, and the fixtures under `fixtures/reference/` come from
webcrack (MIT, Copyright (c) 2023 j4k0xb, see [`fixtures/LICENSE-reference`](fixtures/LICENSE-reference)).
`vendor/regress` is a fork of regress by ridiculousfish (MIT OR Apache-2.0). See [`NOTICE`](NOTICE).
