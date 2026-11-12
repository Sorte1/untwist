class Shape {
  constructor(kind) {
    this.kind = kind;
  }
}
class Box extends Shape {
  constructor(width = 1, height = width * 2, label) {
    super("box");
    this.width = width;
    this.height = height;
    this.label = label;
  }
  area(scale = 1) {
    return this.width * this.height * scale;
  }
}
class Tagged extends Box {
  constructor(tag = "t", ...rest) {
    super(...rest);
    this.tag = tag;
  }
}
function later(step = 1, ...more) {
  const run = () => step + more.length;
  return run();
}
console.log(JSON.stringify(new Box()), JSON.stringify(new Box(3)), JSON.stringify(new Box(3, 4, "x")));
console.log(new Box(2).area(), new Box(2).area(3), Box.length, Box.prototype.area.length);
console.log(JSON.stringify(new Tagged()), JSON.stringify(new Tagged("q", 5)), Tagged.length);
console.log(later(), later(2, 0, 0), later.length);
