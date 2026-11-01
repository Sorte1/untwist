import { say } from "./say.js";
say("eval math");
export const add = (a, b) => a + b;
export const mul = (a, b) => a * b;
export function triple(n) {
  return mul(n, 3);
}
