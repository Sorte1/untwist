class Base {
  constructor(v) {
    this.v = v;
  }
  get val() {
    return this.v;
  }
  set val(x) {
    this.v = x * 2;
  }
  greet(who) {
    return "hi " + who + " " + this.v;
  }
  static make(v) {
    return new this(v);
  }
  static get kind() {
    return "base";
  }
  ["comp" + "uted"]() {
    return "c";
  }
}
class Child extends Base {
  constructor(v, w) {
    super(v);
    this.w = w;
  }
  greet(who) {
    return super.greet(who).toUpperCase() + this.w;
  }
  static make(v) {
    return super.make(v);
  }
}
const c = new Child(1, 2);
console.log(c.greet("x"), c.val, ((c.val = 5), c.v), c.computed());
console.log(Child.make(3) instanceof Child, Child.kind, Object.getPrototypeOf(Child) === Base);
console.log(Base.name, Child.name, typeof Base);
console.log(Object.getOwnPropertyNames(Base.prototype).sort().join(","), Object.keys(Base.prototype).length);
const d = Object.getOwnPropertyDescriptor(Base.prototype, "greet");
console.log(d.enumerable, d.writable, d.configurable);
console.log(Object.getOwnPropertyDescriptor(Base, "make").enumerable);
try {
  Base(1);
} catch (e) {
  console.log(e.constructor.name);
}
const Anon = class {};
console.log(Anon.name);
class Counter {
  constructor() {
    this.n = 0;
  }
  inc() {
    this.n++;
    return this;
  }
}
console.log(new Counter().inc().inc().n);
function Legacy(x) {
  this.x = x;
}
Legacy.prototype.get = function () {
  return this.x;
};
class FromLegacy extends Legacy {
  get() {
    return super.get() + 1;
  }
}
console.log(new FromLegacy(4).get());
