import { log } from "./log.js";
import { isEven } from "./cycle-a.js";
export function isOdd(n) {
  return n === 0 ? false : isEven(n - 1);
}
log("eval cycle-b", typeof isEven);
