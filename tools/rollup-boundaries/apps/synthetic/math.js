import { log } from "./log.js";
export const EPS = 1e-6;
export function clamp(value, low, high) {
  return value < low ? low : value > high ? high : value;
}
export function lerp(a, b, t) {
  return a + (b - a) * clamp(t, 0, 1);
}
function format(value) {
  return value.toFixed(3);
}
export function describe(value) {
  return "num(" + format(value) + ")";
}
log("eval math", describe(lerp(1, 3, 0.25)));
