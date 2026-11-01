var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
var seen = [];
var log = (label) => (seen.push(label), label);
class Parent {
  constructor(a, b) {
    this.sum = (a || 0) + (b || 0);
  }
}
class Kid extends Parent {
  constructor(a) {
    super(...arguments);
    __publicField(this, "x", 1);
  }
}
class Plain {
  constructor(a) {
    __publicField(this, "y", 2);
  }
}
console.log(Kid.length, Plain.length, new Kid(1, 2).sum, new Kid(1, 2).x, new Plain().y);
