var check = function (it) {
  return it && it.Math === Math && it;
};
var root =
  check(typeof globalThis == "object" && globalThis) ||
  check(typeof window == "object" && window) ||
  check(typeof self == "object" && self) ||
  (function () {
    return this;
  })() ||
  Function("return this")();
function ownKeys(object, enumerableOnly) {
  var keys = Object.keys(object);
  if (Object.getOwnPropertySymbols) {
    var symbols = Object.getOwnPropertySymbols(object);
    enumerableOnly &&
      (symbols = symbols.filter(function (sym) {
        return Object.getOwnPropertyDescriptor(object, sym).enumerable;
      })),
      keys.push.apply(keys, symbols);
  }
  return keys;
}
function defineProperty(obj, key, value) {
  Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true });
  return obj;
}
function objectSpread2(target) {
  for (var i = 1; i < arguments.length; i++) {
    var source = null != arguments[i] ? arguments[i] : {};
    i % 2
      ? ownKeys(Object(source), !0).forEach(function (key) {
          defineProperty(target, key, source[key]);
        })
      : Object.getOwnPropertyDescriptors
        ? Object.defineProperties(target, Object.getOwnPropertyDescriptors(source))
        : ownKeys(Object(source)).forEach(function (key) {
            Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key));
          });
  }
  return target;
}
function isNativeReflectConstruct() {
  try {
    var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {}));
  } catch (t) {}
  return (isNativeReflectConstruct = function () {
    return !!t;
  })();
}
function construct(Parent, args, Class) {
  if (isNativeReflectConstruct()) return Reflect.construct.apply(null, arguments);
  var a = [null];
  a.push.apply(a, args);
  return new (Parent.bind.apply(Parent, a))();
}
var words = ["alpha", "beta", "gamma"];
var merged = objectSpread2({}, { first: words[0] }, { second: words[1] });
var made = construct(Array, [3]);
function lookup(name) {
  return root[name];
}
var promise = lookup(["Pro", "mise"].join(""));
console.log(typeof root.Math, typeof promise, words.join("-"), merged.first + merged.second, made.length);
