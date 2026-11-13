import { Shape } from "./shape.js";
import { log } from "./log.js";
export default class Square extends Shape {
  constructor(side) {
    super("square");
    this.side = side;
  }
  area() {
    return this.side * this.side;
  }
}
log("eval square");
