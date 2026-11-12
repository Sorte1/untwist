"use strict";

const log = [];
const later = v => new Promise(resolve => setTimeout(() => resolve(v), 0));
async function fetchTwice(x) {
  log.push("start " + x);
  const a = await later(x);
  const b = await later(a + 1);
  return a + b;
}
async function guarded() {
  try {
    await later(0);
    throw new TypeError("t");
  } catch (e) {
    log.push("caught " + e.name);
    return "r";
  } finally {
    log.push("finally");
  }
}
const service = {
  base: 100,
  async total(n) {
    const add = async k => this.base + k + arguments.length;
    let sum = 0;
    for (let i = 0; i < n; i++) sum += await add(i);
    return sum;
  }
};
class Loader {
  constructor() {
    this.items = [];
  }
  async load(v) {
    await null;
    this.items.push(v);
    return this.items.length;
  }
}
async function main() {
  const p = fetchTwice(1);
  log.push("after call");
  console.log(await p, await guarded(), await service.total(3));
  const l = new Loader();
  console.log(await l.load("a"), await l.load("b"));
  const arrow = async n => (await later(n)) * 2;
  console.log(await arrow(4), (await Promise.all([arrow(1), fetchTwice(2)])).join(","));
  console.log(log.join("; "));
}
main().catch(e => console.log("fail", e));
