import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { MODULE, compileModule, freshInstance, packageDir } from './instance.mjs';

const STACK_POINTER_GLOBAL = 0;
const GLOBAL_SECTION = 6;
const I32_CONST = 0x41;

function leb(bytes, at) {
  let value = 0;
  let shift = 0;
  let index = at;
  let byte;
  do {
    byte = bytes[index];
    value += (byte & 0x7f) * 2 ** shift;
    shift += 7;
    index += 1;
  } while (byte & 0x80);
  return [value, index];
}

function sleb(bytes, at) {
  let [value, index] = leb(bytes, at);
  const bits = 7 * (index - at);
  if (bits < 53 && bytes[index - 1] & 0x40) {
    value -= 2 ** bits;
  }
  return [value, index];
}

function globalInits(bytes) {
  let at = 8;
  while (at < bytes.length) {
    const id = bytes[at];
    const [size, body] = leb(bytes, at + 1);
    if (id === GLOBAL_SECTION) {
      return readGlobals(bytes, body);
    }
    at = body + size;
  }
  return [];
}

function readGlobals(bytes, start) {
  let [count, at] = leb(bytes, start);
  const inits = [];
  for (; count > 0; count -= 1) {
    at += 2;
    const op = bytes[at];
    const [value, after] = op === I32_CONST ? sleb(bytes, at + 1) : [null, at + 1];
    inits.push(value);
    at = bytes.indexOf(0x0b, after) + 1;
  }
  return inits;
}

const compiled = compileModule();
const instance = await freshInstance(compiled);
const stack = globalInits(readFileSync(join(packageDir(), MODULE)))[STACK_POINTER_GLOBAL];
console.log(`${instance.nestingBudget()} ${instance.shadowStackBytes()} ${stack} ${instance.callDepthCap()} ${instance.evaluatorNestingCap()}`);
