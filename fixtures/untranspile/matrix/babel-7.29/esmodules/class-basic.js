"use strict";

class Point {
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }
  get length() {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  }
  set length(v) {
    var k = v / this.length;
    this.x *= k;
    this.y *= k;
  }
  add(other) {
    return new Point(this.x + other.x, this.y + other.y);
  }
  ["to" + "Pair"]() {
    return [this.x, this.y];
  }
  static origin() {
    return new this(0, 0);
  }
  static get unit() {
    return new Point(1, 0);
  }
}
var p = new Point(3, 4);
console.log(p.length, p.add(Point.origin()).toPair().join(","), Point.unit.x);
p.length = 10;
console.log(p.x, p.y);
console.log(typeof Point, Point.name);
var Anon = class {
  hi() {
    return "anon";
  }
};
console.log(new Anon().hi(), Anon.name);
