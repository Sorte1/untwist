import { Shape } from "./shape.js";
import { log } from "./log.js";
const PI = Math.PI;
export class Circle extends Shape {
  constructor(r) {
    super("circle");
    this.r = r;
  }
  area() {
    return PI * this.r * this.r;
  }
}
log("eval circle");
