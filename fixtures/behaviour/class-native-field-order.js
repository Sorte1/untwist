var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
var seen = [];
var log = (label) => (seen.push(label), label);
class Ordered {
  y = log("y");
  constructor() {
    __publicField(this, "x", log("x"));
  }
  z = log("z");
}
var o = new Ordered();
console.log(seen.join(","), Object.keys(o).join(","));
