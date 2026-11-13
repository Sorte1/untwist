import { log } from "./log.js";
export let count = 0;
export function increment(step = 1) {
  count += step;
  return count;
}
export function reset() {
  count = 0;
}
log("eval counter", count);
