var __defProp = Object.defineProperty;
var __defProps = Object.defineProperties;
var __getOwnPropDescs = Object.getOwnPropertyDescriptors;
var __getOwnPropSymbols = Object.getOwnPropertySymbols;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __propIsEnum = Object.prototype.propertyIsEnumerable;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __spreadValues = (a2, b2) => {
  for (var prop in b2 || (b2 = {}))
    if (__hasOwnProp.call(b2, prop))
      __defNormalProp(a2, prop, b2[prop]);
  if (__getOwnPropSymbols)
    for (var prop of __getOwnPropSymbols(b2)) {
      if (__propIsEnum.call(b2, prop))
        __defNormalProp(a2, prop, b2[prop]);
    }
  return a2;
};
var __spreadProps = (a2, b2) => __defProps(a2, __getOwnPropDescs(b2));
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
function sum() {
  var total = 0;
  for (var i = 0; i < arguments.length; i++) total += arguments[i];
  return total;
}
const a = [1, 2, 3];
const b = [...a, 4, ...[5, 6]];
console.log(b.join(","), b.length);
console.log(sum(...a, 10, ...[20]));
console.log(Math.max(.../* @__PURE__ */ new Set([3, 9, 1])));
const holes = [, 1];
const copy = [...holes];
console.log(0 in copy, copy.length);
const o1 = { x: 1, y: 2, get z() {
  return 3;
} };
const o2 = __spreadProps(__spreadValues({}, o1), { y: 5, w: 6 });
const zd = Object.getOwnPropertyDescriptor(o2, "z");
console.log(JSON.stringify(o2), Object.keys(o2).join(","), zd.enumerable, typeof zd.get);
const s = Symbol("k");
const o3 = { [s]: 1, a: 2 };
const o4 = __spreadValues(__spreadValues(__spreadValues(__spreadValues({}, o3), null), void 0), "pq");
console.log(o4[s], Object.getOwnPropertySymbols(o4).length, JSON.stringify(o4));
function rest(first, ...others2) {
  return first + ":" + others2.join("|") + ":" + others2.length;
}
console.log(rest(1), rest(1, 2, 3), rest.length);
const _a = o2, { x } = _a, others = __objRest(_a, ["x"]);
console.log(x, JSON.stringify(others));
function callsThis(...args) {
  return this.base + args.length;
}
const holder = { base: 100, m: callsThis };
console.log(holder.m(...a), holder["m"](...a, ...a));
try {
  console.log([...void 0]);
} catch (e) {
  console.log(e.constructor.name);
}
console.log(new Date(...[2020, 0, 2]).getFullYear());
const order = [];
const tap = (v) => (order.push(v), [v]);
console.log(sum(...tap(1), ...tap(2)), order.join(","));
try {
  console.log([..."xy"].join("-"));
} catch (e) {
  console.log(e.constructor.name);
}
