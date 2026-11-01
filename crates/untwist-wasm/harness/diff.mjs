import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

import { compileModule, runIsolated, runRefused } from './instance.mjs';

const ADMISSION_REFUSAL = /^warning: input 1 was not processed: [^\n]*\n$/;

function cases(manifest, shard, shards) {
  return readFileSync(manifest, 'utf8')
    .split('\n')
    .filter((line) => line !== '')
    .map((line) => line.split('\t'))
    .filter((_, index) => index % shards === shard)
    .map(([id, flags, input, stdout, stderr, code]) => ({ id, flags, input, stdout, stderr, code: Number(code) }));
}

function pinsFrom(file) {
  if (file === undefined) {
    return new Map();
  }
  return new Map(
    readFileSync(file, 'utf8')
      .split('\n')
      .filter((line) => line !== '')
      .map((line) => line.split('\t'))
      .map(([id, limits, digest]) => [id, { limits, digest }]),
  );
}

function digest(text) {
  return createHash('sha256').update(text).digest('hex').slice(0, 16);
}

function difference(expected, actual) {
  if (actual.stdout !== readFileSync(expected.stdout, 'utf8')) {
    return 'stdout';
  }
  if (actual.stderr !== readFileSync(expected.stderr, 'utf8')) {
    return 'stderr';
  }
  return actual.code === expected.code ? null : `exit ${actual.code}, native ${expected.code}`;
}

async function passedThrough(compiled, source, expected, actual) {
  const fired = actual.limits.split(',');
  if (!fired.includes('refused') || !fired.includes('budget')) {
    return false;
  }
  if (actual.code !== 3 || !ADMISSION_REFUSAL.test(actual.stderr)) {
    return false;
  }
  return actual.stdout === (await runRefused(compiled, source, expected.flags)).stdout;
}

function pinned(pins, expected, actual) {
  const pin = pins.get(expected.id);
  return pin !== undefined
    && pin.limits === actual.limits
    && pin.digest === digest(actual.stdout)
    && actual.stderr === readFileSync(expected.stderr, 'utf8')
    && actual.code === expected.code;
}

async function explained(context, expected, actual) {
  if (await passedThrough(context.compiled, context.source, expected, actual)) {
    return true;
  }
  return pinned(context.pins, expected, actual);
}

async function verdict(context, expected, actual) {
  if (actual.trap?.kind === 'stack') {
    return ['stack-trap', actual.trap.message];
  }
  if (actual.trap !== null) {
    return ['differ', `trap ${actual.trap.message}`];
  }
  const what = difference(expected, actual);
  if (what === null) {
    return ['identical', ''];
  }
  const detail = `${actual.limits}: ${what}, wasm ${digest(actual.stdout)}; native exit ${expected.code}`;
  if (actual.limits !== '' && (await explained(context, expected, actual))) {
    return ['refusal', detail];
  }
  return ['differ', detail];
}

function faked(actual, id, fakes) {
  if (actual.trap !== null || !(fakes.has(id) || fakes.has('all'))) {
    return actual;
  }
  return { ...actual, stdout: `${actual.stdout}0;\n` };
}

async function main() {
  const [manifest, shard = '0', shards = '1'] = process.argv.slice(2);
  const compiled = compileModule();
  const pins = pinsFrom(process.env.UNTWIST_WASM_PINS);
  const fakes = new Set((process.env.UNTWIST_WASM_FAKE ?? '').split(',').filter((id) => id !== ''));
  const counts = { identical: 0, 'stack-trap': 0, differ: 0, refusal: 0 };
  for (const expected of cases(manifest, Number(shard), Number(shards))) {
    const source = readFileSync(expected.input, 'utf8');
    const actual = faked(await runIsolated(compiled, source, expected.flags), expected.id, fakes);
    const [kind, detail] = await verdict({ compiled, source, pins }, expected, actual);
    counts[kind] += 1;
    console.log(`${kind}\t${expected.id}\t${detail}`);
    globalThis.gc?.();
  }
  console.log(`counts\t${counts.identical}\t${counts['stack-trap']}\t${counts.differ}\t${counts.refusal}`);
  return counts.differ === 0 ? 0 : 1;
}

process.exitCode = await main();
