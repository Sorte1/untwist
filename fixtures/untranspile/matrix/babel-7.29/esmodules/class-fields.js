"use strict";

var _Counter;
var _ref;
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == typeof i ? i : i + ""; }
function _toPrimitive(t, e) { if ("object" != typeof t || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != typeof i) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
var seq = 0;
_ref = "dyn" + seq;
class Counter {
  constructor(step) {
    _defineProperty(this, "count", 0);
    _defineProperty(this, "step", 1);
    _defineProperty(this, "id", ++seq);
    _defineProperty(this, "label", "c" + this.id);
    _defineProperty(this, _ref, "dynamic");
    _defineProperty(this, "inc", () => {
      this.count += this.step;
      return this;
    });
    if (step) this.step = step;
    Counter.instances++;
  }
}
_Counter = Counter;
_defineProperty(Counter, "instances", 0);
_defineProperty(Counter, "created", _Counter.instances + 1);
var c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
var inc = c.inc;
inc();
console.log(c.count, Object.keys(c).join(","));
class Base {
  constructor() {
    this.setup();
  }
  setup() {}
}
class Derived extends Base {
  constructor() {
    super(...arguments);
    _defineProperty(this, "value", 5);
  }
  setup() {
    this.seen = this.value;
  }
}
var dd = new Derived();
console.log(dd.seen, dd.value);
class Shadow {
  set x(v) {
    this.setterRan = true;
  }
}
class Defines extends Shadow {
  constructor() {
    super(...arguments);
    _defineProperty(this, "x", 1);
  }
}
var df = new Defines();
console.log(df.x, df.setterRan === true);
