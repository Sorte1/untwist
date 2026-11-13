import { log } from "./log.js";
import { isOdd } from "./cycle-b.js";
export function isEven(n) {
  return n === 0 ? true : isOdd(n - 1);
}
log("eval cycle-a", isEven(4));
