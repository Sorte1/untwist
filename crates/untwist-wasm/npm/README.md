# untwist

[Untwist](https://github.com/Sorte1/untwist) is a JavaScript deobfuscator, unminifier and
unbundler written in Rust. This package is its WebAssembly build, the engine behind the playground
at [untwist.dev](https://untwist.dev). For the same flags its output matches the `untwist` CLI byte
for byte. The input is never executed: string decoders run in a sandboxed, step-budgeted
interpreter.

```sh
npm install untwist
```

## Usage

The package is an ES module built with `wasm-bindgen --target web`. Call `init` once first.

```js
import init, { run } from 'untwist';

await init();
const result = run('var a = !0;', '--no-unpack', false);
console.log(result.stdout); // "var a = true;\n"
```

In Node, pass the module bytes to `init`:

```js
import { readFile } from 'node:fs/promises';
import init, { run } from 'untwist';

const wasm = await readFile(new URL(import.meta.resolve('untwist/untwist_wasm_bg.wasm')));
await init({ module_or_path: wasm });
const result = run(source, '--unsafe', false);
```

## API

`run(source, flags, refused)` processes one script and returns a `RunOutput`. `flags` is the CLI's
flags as one string: `--unsafe`, `--no-deobfuscate`, `--no-unminify`, `--no-unpack`, `--no-jsx`,
`--single-file`, `--keep-human-names`, `--rename none|simple|unique|mangle`, `-m`,
`--max-iterations N`, `--fuel N`, `--max-memory SIZE`, `--timeout SECS` and
`--disable PASS[,PASS]`. Pass `false` for `refused`; `true` returns the input unchanged.

`--max-memory` and `--timeout` stop an input between its passes, keeping the output so far; one
long pass is not cut short, since nothing can stop wasm code from outside but its host. For input
you do not trust, call `run` in a Worker you can terminate. After `run` throws a
`WebAssembly.RuntimeError` or a `RangeError` (a trap, such as running out of stack or memory),
throw that instance away and load the module again in a new Worker: a trapped instance keeps its
memory and its budget state as they were at the trap.

| `RunOutput` field | Meaning |
| --- | --- |
| `stdout` | the processed code |
| `stderr` | warnings and diagnostics |
| `code` | exit code: `0` success, `1` error, `2` bad flags, `3` input too deep |
| `limits` | comma-separated limits that fired (`refused`, `budget`, `unpack-text`, `eval-cap`), or empty |

Call `free()` on the result when done. `pass_table()` returns every pass as JSON, with its stage
and whether it is unsafe.

By default a string-array call is decoded only when Untwist can prove from the program text that
nothing changes what the decoder returns. `--unsafe` assumes built-ins are not patched and enables
the heuristic `dead-code`, `template-literals` and JSX passes.

## License

MIT OR Apache-2.0.
