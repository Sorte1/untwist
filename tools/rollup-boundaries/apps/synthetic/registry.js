import { log } from "./log.js";
import { increment } from "./counter.js";
class Registry {
  constructor() {
    this.items = new Map();
  }
  add(key, value) {
    increment();
    this.items.set(key, value);
    return this;
  }
  keys() {
    return [...this.items.keys()];
  }
}
const registry = new Registry();
export default registry;
log("eval registry");
