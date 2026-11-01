import { readFileSync, writeSync } from 'node:fs';

import { compileModule, runIsolated } from './instance.mjs';

const VALUE_FLAGS = new Set(['--rename', '--max-iterations', '--fuel', '-j', '--jobs']);
const USAGE = 2;

function parseArgs(argv) {
  const flags = [];
  const files = [];
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (VALUE_FLAGS.has(arg)) {
      flags.push(arg, argv[index + 1] ?? '');
      index += 1;
    } else if (arg.startsWith('-')) {
      flags.push(arg);
    } else {
      files.push(arg);
    }
  }
  return { flags: flags.join(' '), files };
}

function writeAll(fd, text) {
  const bytes = Buffer.from(text, 'utf8');
  let offset = 0;
  while (offset < bytes.length) {
    offset += writeSync(fd, bytes, offset);
  }
}

async function main() {
  const { flags, files } = parseArgs(process.argv.slice(2));
  if (files.length > 1) {
    writeAll(2, 'error: the wasm harness takes one input\n');
    return USAGE;
  }
  const source = readFileSync(files.length === 1 ? files[0] : 0, 'utf8');
  const compiled = compileModule();
  const result = await runIsolated(compiled, source, flags);
  writeAll(1, result.stdout);
  if (result.trap !== null) {
    writeAll(2, `wasm trap (${result.trap.kind}): ${result.trap.message}\n`);
  }
  writeAll(2, result.stderr);
  if (process.env.UNTWIST_WASM_STATS === '1') {
    const ms = result.runMs.toFixed(1);
    writeAll(2, `wasm-stats run-ms ${ms} memory-bytes ${result.memoryBytes} module-bytes ${compiled.size}\n`);
  }
  return result.code;
}

process.exitCode = await main();
