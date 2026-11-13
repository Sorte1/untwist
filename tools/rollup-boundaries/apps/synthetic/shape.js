import { log } from "./log.js";
import { describe } from "./math.js";
let created = 0;
export class Shape {
  constructor(name) {
    this.name = name;
    created++;
  }
  area() {
    return 0;
  }
  toString() {
    return this.name + " " + describe(this.area());
  }
}
export function createdShapes() {
  return created;
}
log("eval shape");
