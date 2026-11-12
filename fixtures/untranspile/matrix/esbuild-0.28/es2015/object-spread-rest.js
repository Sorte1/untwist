var __defProp = Object.defineProperty;
var __defProps = Object.defineProperties;
var __getOwnPropDescs = Object.getOwnPropertyDescriptors;
var __getOwnPropSymbols = Object.getOwnPropertySymbols;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __propIsEnum = Object.prototype.propertyIsEnumerable;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __spreadValues = (a2, b) => {
  for (var prop in b || (b = {}))
    if (__hasOwnProp.call(b, prop))
      __defNormalProp(a2, prop, b[prop]);
  if (__getOwnPropSymbols)
    for (var prop of __getOwnPropSymbols(b)) {
      if (__propIsEnum.call(b, prop))
        __defNormalProp(a2, prop, b[prop]);
    }
  return a2;
};
var __spreadProps = (a2, b) => __defProps(a2, __getOwnPropDescs(b));
var __restKey = (key) => typeof key === "symbol" ? key : key + "";
var __objRest = (source, exclude) => {
  var target = {};
  for (var prop in source)
    if (__hasOwnProp.call(source, prop) && exclude.indexOf(prop) < 0)
      target[prop] = source[prop];
  if (source != null && __getOwnPropSymbols)
    for (var prop of __getOwnPropSymbols(source)) {
      if (exclude.indexOf(prop) < 0 && __propIsEnum.call(source, prop))
        target[prop] = source[prop];
    }
  return target;
};
const base = { a: 1, b: 2 };
const extra = { c: 3 };
const merged = __spreadProps(__spreadValues(__spreadValues({}, base), extra), { d: 4 });
const overridden = __spreadProps(__spreadValues({}, base), { a: 9 });
console.log(JSON.stringify(merged), JSON.stringify(overridden));
const _a = merged, { a } = _a, others = __objRest(_a, ["a"]);
console.log(a, JSON.stringify(others));
function without(_b) {
  var _c = _b, { secret } = _c, pub = __objRest(_c, ["secret"]);
  return pub;
}
console.log(JSON.stringify(without({ secret: 1, open: 2 })));
const withGetter = __spreadValues({}, { get g() {
  return "got";
} });
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
const sym = /* @__PURE__ */ Symbol("s");
const symObj = __spreadValues({}, { [sym]: 1, plain: 2 });
console.log(symObj[sym], symObj.plain);
const _d = symObj, { [sym]: picked } = _d, restSym = __objRest(_d, [__restKey(sym)]);
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(__spreadValues(__spreadValues(__spreadValues(__spreadValues({}, null), void 0), "hi"), [5])));
let order = [];
const tracked = { get x() {
  order.push("x");
  return 1;
}, get y() {
  order.push("y");
  return 2;
} };
const copy = __spreadValues({}, tracked);
console.log(order.join(","), copy.x + copy.y);
