class Shape {
  constructor(kind) {
    this.kind = kind;
  }
  area() {
    return 0;
  }
  static of(kind) {
    return new Shape(kind);
  }
}
class Square extends Shape {
  constructor(side) {
    super("square");
    this.side = side;
  }
  area() {
    return this.side * this.side;
  }
}
const s = new Square(3);
const seen = [];
for (const key in s) seen.push(key);
console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
for (const make of [() => Shape("x"), () => Square(1)]) {
  try {
    make();
    console.log("called without new");
  } catch (e) {
    console.log(e.constructor.name);
  }
}
console.log(Shape.length, Square.length, typeof Shape.prototype.area.prototype);
