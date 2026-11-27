import add, { twice } from "./esm.mjs";
import * as both from "./esm.mjs";
const cjs = require("./cjs.cjs");
const data = require("./data.json");
export const total = add(cjs.one(), twice(data.n)) + Object.keys(both).length;
export default function main() {
  return import("./late.mjs").then((late) => late.value + total);
}
globalThis.__runtime = main;
