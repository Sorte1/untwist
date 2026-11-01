var __spreadArray = this && this.__spreadArray || function (to, from, pack) {
  if (pack || arguments.length === 2) {
    for (var i = 0, l = from.length, ar; i < l; i++) {
      if (ar || !(i in from)) {
        if (!ar) {
          ar = Array.prototype.slice.call(from, 0, i);
        }
        ar[i] = from[i];
      }
    }
  }
  return to.concat(ar || Array.prototype.slice.call(from));
};
var __defProp = Object.defineProperty;
var __defProps = Object.defineProperties;
var __getOwnPropDescs = Object.getOwnPropertyDescriptors;
var __getOwnPropSymbols = Object.getOwnPropertySymbols;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __propIsEnum = Object.prototype.propertyIsEnumerable;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, {
  enumerable: true,
  configurable: true,
  writable: true,
  value
}) : obj[key] = value;
var __spreadValues = (a2, b2) => {
  for (var prop in b2 ||= {}) {
    if (__hasOwnProp.call(b2, prop)) {
      __defNormalProp(a2, prop, b2[prop]);
    }
  }
  if (__getOwnPropSymbols) {
    for (var prop of __getOwnPropSymbols(b2)) {
      if (__propIsEnum.call(b2, prop)) {
        __defNormalProp(a2, prop, b2[prop]);
      }
    }
  }
  return a2;
};
var __spreadProps = (a2, b2) => __defProps(a2, __getOwnPropDescs(b2));
var xs = [1, 2];
var safeArray = __spreadArray(__spreadArray([], [0], false), [3, 4], false);
var array = __spreadArray(__spreadArray([], xs, true), [5], false);
var safeObject = __spreadValues(__spreadValues({}, { a: 1 }), { b: 2 });
var object = __spreadProps(__spreadValues({}, safeObject), { c: 3 });
console.log(safeArray.join(), array.join());
console.log(JSON.stringify(safeObject), JSON.stringify(object));
