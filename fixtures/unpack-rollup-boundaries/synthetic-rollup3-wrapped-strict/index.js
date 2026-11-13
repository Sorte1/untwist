(function () {
  "use strict";
  var hostConfig;
  var hostHook;
(function () {
  'use strict';

  const lines = [];
  function log(...parts) {
    lines.push(parts.join(" "));
    console.log(...parts);
  }
  log("eval log");

  const EPS = 1e-6;
  function clamp(value, low, high) {
    return value < low ? low : value > high ? high : value;
  }
  function lerp(a, b, t) {
    return a + (b - a) * clamp(t, 0, 1);
  }
  function format$1(value) {
    return value.toFixed(3);
  }
  function describe(value) {
    return "num(" + format$1(value) + ")";
  }
  log("eval math", describe(lerp(1, 3, 0.25)));

  function format(text) {
    return "[" + text + "]";
  }
  function pad(text, width) {
    let out = String(text);
    while (out.length < width) out = " " + out;
    return format(out);
  }
  const SEPARATOR = " | ";
  log("eval strings", pad("x", 3));

  let count = 0;
  function increment(step = 1) {
    count += step;
    return count;
  }
  function reset() {
    count = 0;
  }
  log("eval counter", count);

  const red = "#f00";
  const green = "#0f0";
  const blue = "#00f";
  function mix(a, b) {
    return a + "+" + b;
  }

  var colors = /*#__PURE__*/Object.freeze({
    __proto__: null,
    blue: blue,
    green: green,
    mix: mix,
    red: red
  });

  function isOdd(n) {
    return n === 0 ? false : isEven(n - 1);
  }
  log("eval cycle-b", typeof isEven);

  function isEven(n) {
    return n === 0 ? true : isOdd(n - 1);
  }
  log("eval cycle-a", isEven(4));

  let created = 0;
  class Shape {
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
  function createdShapes() {
    return created;
  }
  log("eval shape");

  const PI = Math.PI;
  class Circle extends Shape {
    constructor(r) {
      super("circle");
      this.r = r;
    }
    area() {
      return PI * this.r * this.r;
    }
  }
  log("eval circle");

  class Square extends Shape {
    constructor(side) {
      super("square");
      this.side = side;
    }
    area() {
      return this.side * this.side;
    }
  }
  log("eval square");

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
  log("eval registry");

  const handlers = {};
  function on(name, handler) {
    (handlers[name] ||= []).push(handler);
  }
  function emit(name, ...args) {
    for (const handler of handlers[name] || []) handler(...args);
  }
  on("boot", (who) => log("boot handler", who));
  log("eval events");

  const pick = (name) => colors[name];
  const shapes = [new Circle(1), new Square(2)];
  registry.add("circle", shapes[0]).add("square", shapes[1]);
  increment(5);
  log("count", count, "shapes", createdShapes(), "even", isEven(10));
  log("colors", Object.keys(colors).join(","), pick("green"), mix(red, blue));
  log(shapes.map(String).join(SEPARATOR), pad(lerp(0, 10, 0.5), 6), EPS);
  on("boot", () => log("main boot"));
  emit("boot", "app");
  reset();
  log("after reset", count, registry.keys().join(","), lines.length);

})();
//# sourceMappingURL=main.js.map
  hostConfig = { mode: "late" };
  hostHook = function (n) {
    return "hooked " + n;
  };
})();
