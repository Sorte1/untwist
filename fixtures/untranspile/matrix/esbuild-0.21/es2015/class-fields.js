var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
var _a;
let seq = 0;
_a = "dyn" + seq;
const _Counter = class _Counter {
  constructor(step) {
    __publicField(this, "count", 0);
    __publicField(this, "step", 1);
    __publicField(this, "id", ++seq);
    __publicField(this, "label", "c" + this.id);
    __publicField(this, _a, "dynamic");
    __publicField(this, "inc", () => {
      this.count += this.step;
      return this;
    });
    if (step) this.step = step;
    _Counter.instances++;
  }
};
__publicField(_Counter, "instances", 0);
__publicField(_Counter, "created", _Counter.instances + 1);
let Counter = _Counter;
const c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
const { inc } = c;
inc();
console.log(c.count, Object.keys(c).join(","));
class Base {
  constructor() {
    this.setup();
  }
  setup() {
  }
}
class Derived extends Base {
  constructor() {
    super(...arguments);
    __publicField(this, "value", 5);
  }
  setup() {
    this.seen = this.value;
  }
}
const dd = new Derived();
console.log(dd.seen, dd.value);
class Shadow {
  set x(v) {
    this.setterRan = true;
  }
}
class Defines extends Shadow {
  constructor() {
    super(...arguments);
    __publicField(this, "x", 1);
  }
}
const df = new Defines();
console.log(df.x, df.setterRan === true);
