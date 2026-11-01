import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const DEFAULT_PKG = resolve(HERE, '../../../target/wasm-pkg');
const GLUE = 'untwist_wasm.js';
export const MODULE = 'untwist_wasm_bg.wasm';
const STACK_TRAPS = [
  (error) => error instanceof RangeError && /Maximum call stack size exceeded/.test(error.message),
  (error) => error instanceof WebAssembly.RuntimeError && /memory access out of bounds/.test(error.message),
];

let made = 0;

export function packageDir() {
  return resolve(process.env.UNTWIST_WASM_PKG ?? DEFAULT_PKG);
}

export function compileModule(pkg = packageDir()) {
  const bytes = readFileSync(join(pkg, MODULE));
  return { pkg, module: new WebAssembly.Module(bytes), size: bytes.length };
}

export async function freshInstance(compiled) {
  made += 1;
  const url = `${pathToFileURL(join(compiled.pkg, GLUE)).href}?instance=${made}`;
  const glue = await import(url);
  const exports = glue.initSync({ module: compiled.module });
  return {
    run: glue.run,
    memory: exports.memory,
    nestingBudget: glue.nesting_budget,
    shadowStackBytes: glue.shadow_stack_bytes,
    callDepthCap: glue.call_depth_cap,
    evaluatorNestingCap: glue.evaluator_nesting_cap,
  };
}

export function trapKind(error) {
  if (STACK_TRAPS.some((matches) => matches(error))) {
    return 'stack';
  }
  return error instanceof WebAssembly.RuntimeError ? 'other' : null;
}

function taken(output) {
  const result = { stdout: output.stdout, stderr: output.stderr, code: output.code, limits: output.limits };
  output.free();
  return result;
}

export async function runIsolated(compiled, source, flags) {
  const instance = await freshInstance(compiled);
  const started = performance.now();
  try {
    const result = taken(instance.run(source, flags, false));
    const runMs = performance.now() - started;
    return { ...result, trap: null, runMs, memoryBytes: instance.memory.buffer.byteLength };
  } catch (error) {
    const kind = trapKind(error);
    if (kind === null) {
      throw error;
    }
    const runMs = performance.now() - started;
    const memoryBytes = instance.memory.buffer.byteLength;
    const refused = await runRefused(compiled, source, flags);
    return { ...refused, trap: { kind, message: `${error.name}: ${error.message}` }, runMs, memoryBytes };
  }
}

export async function runRefused(compiled, source, flags) {
  return taken((await freshInstance(compiled)).run(source, flags, true));
}
