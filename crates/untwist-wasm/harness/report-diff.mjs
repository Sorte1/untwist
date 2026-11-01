import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { compileModule, trapKind } from './instance.mjs';

const GLUE = 'untwist_wasm.js';
const FIELDS = ['stdout', 'stderr', 'code', 'limits'];

let made = 0;

function cases(manifest, shard, shards) {
  return readFileSync(manifest, 'utf8')
    .split('\n')
    .filter((line) => line !== '')
    .map((line) => line.split('\t'))
    .filter((_, index) => index % shards === shard)
    .map(([id, flags, input]) => ({ id, flags, input }));
}

async function glue(compiled) {
  made += 1;
  const module = await import(`${pathToFileURL(join(compiled.pkg, GLUE)).href}?report=${made}`);
  module.initSync({ module: compiled.module });
  return module;
}

function taken(output) {
  const result = Object.fromEntries(FIELDS.map((field) => [field, output[field]]));
  output.free();
  return result;
}

async function attempt(compiled, entry, source, flags, refused) {
  const instance = await glue(compiled);
  try {
    return { ...taken(instance[entry](source, flags, refused)), trap: null };
  } catch (error) {
    const kind = trapKind(error);
    if (kind === null) {
      throw error;
    }
    return { trap: kind };
  }
}

async function both(compiled, source, flags, refused) {
  const expected = await attempt(compiled, 'run', source, flags, refused);
  const actual = await attempt(compiled, 'run_report', source, flags, refused);
  globalThis.gc?.();
  return { expected, actual };
}

function difference({ expected, actual }) {
  if (expected.trap !== actual.trap) {
    return `trap ${actual.trap}, run ${expected.trap}`;
  }
  return FIELDS.find((field) => expected[field] !== actual[field]) ?? null;
}

async function verdict(compiled, { flags, input }) {
  const source = readFileSync(input, 'utf8');
  const first = await both(compiled, source, flags, false);
  const differs = difference(first);
  if (differs !== null || first.expected.trap === null) {
    return { verdict: differs === null ? 'identical' : 'differ', why: differs };
  }
  const refused = difference(await both(compiled, source, flags, true));
  return { verdict: refused === null ? 'trapped' : 'differ', why: refused };
}

async function main() {
  const [manifest, shard = '0', shards = '1'] = process.argv.slice(2);
  const compiled = compileModule();
  const counts = { identical: 0, trapped: 0, differ: 0 };
  const all = cases(manifest, Number(shard), Number(shards));
  for (const item of all) {
    const { verdict: kind, why } = await verdict(compiled, item);
    counts[kind] += 1;
    if (kind === 'differ') {
      console.log(`differ\t${item.id}\t${why}`);
    }
  }
  console.log(`report-diff: ${all.length} cases, ${counts.identical} identical, ${counts.trapped} both trapped, ${counts.differ} differ`);
  return counts.differ === 0 ? 0 : 1;
}

process.exitCode = await main();
