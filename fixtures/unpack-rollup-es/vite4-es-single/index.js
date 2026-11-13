import require$$0$1 from "node:path";
function _mergeNamespaces(n, m) {
  for (var i = 0; i < m.length; i++) {
    const e = m[i];
    if (typeof e !== "string" && !Array.isArray(e)) {
      for (const k in e) {
        if (k !== "default" && !(k in n)) {
          const d = Object.getOwnPropertyDescriptor(e, k);
          if (d) {
            Object.defineProperty(n, k, d.get ? d : {
              enumerable: true,
              get: () => e[k]
            });
          }
        }
      }
    }
  }
  return Object.freeze(Object.defineProperty(n, Symbol.toStringTag, { value: "Module" }));
}
function getDefaultExportFromCjs(x) {
  return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, "default") ? x["default"] : x;
}
function getAugmentedNamespace(n) {
  if (n.__esModule)
    return n;
  var f = n.default;
  if (typeof f == "function") {
    var a = function a2() {
      if (this instanceof a2) {
        return Reflect.construct(f, arguments, this.constructor);
      }
      return f.apply(this, arguments);
    };
    a.prototype = f.prototype;
  } else
    a = {};
  Object.defineProperty(a, "__esModule", { value: true });
  Object.keys(n).forEach(function(k) {
    var d = Object.getOwnPropertyDescriptor(n, k);
    Object.defineProperty(a, k, d.get ? d : {
      enumerable: true,
      get: function() {
        return n[k];
      }
    });
  });
  return a;
}
var greet$2 = { exports: {} };
console.log("eval greet");
const path = require$$0$1;
function greet(name2) {
  return "hello " + path.basename(name2);
}
greet.shout = function(name2) {
  return greet(name2).toUpperCase();
};
greet$2.exports = greet;
var shout = greet$2.exports.shout = greet.shout;
var greetExports = greet$2.exports;
const greet$1 = /* @__PURE__ */ getDefaultExportFromCjs(greetExports);
const name = "fixture";
const items$1 = [
  1,
  2,
  3,
  4,
  5
];
const data = {
  name,
  items: items$1
};
var cycleA = {};
var cycleB = {};
var hasRequiredCycleB;
function requireCycleB() {
  if (hasRequiredCycleB)
    return cycleB;
  hasRequiredCycleB = 1;
  console.log("eval cycle-b");
  var a = requireCycleA();
  cycleB.seen = "b saw " + a.early + "," + a.late;
  cycleB.check = function() {
    return "b later " + a.late;
  };
  return cycleB;
}
var hasRequiredCycleA;
function requireCycleA() {
  if (hasRequiredCycleA)
    return cycleA;
  hasRequiredCycleA = 1;
  console.log("eval cycle-a start");
  cycleA.early = "a-early";
  var b = requireCycleB();
  cycleA.late = "a-late";
  cycleA.report = function() {
    return b.seen + " " + b.check();
  };
  console.log("eval cycle-a end");
  return cycleA;
}
var cycleAExports = requireCycleA();
console.log("eval util");
const label = (name2) => `[${name2.padStart(8, ".")}]`;
function total(list) {
  return list.reduce((sum, n) => sum + n, 0);
}
class Counter {
  constructor() {
    this.value = 0;
  }
  add(n) {
    this.value += n;
  }
}
var math = {};
console.log("eval math");
var twice = math.twice = function(n) {
  return n * 2;
};
var half = math.half = (n) => n / 2;
const own = "own";
const barrel = /* @__PURE__ */ _mergeNamespaces({
  __proto__: null,
  half,
  own,
  twice
}, [math]);
console.log("eval shape");
const kind = "square";
function area(side) {
  return side * side;
}
const shape$1 = "shape-default";
const shape$2 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  area,
  default: shape$1,
  kind
}, Symbol.toStringTag, { value: "Module" }));
const require$$0 = /* @__PURE__ */ getAugmentedNamespace(shape$2);
console.log("eval bridge");
const shape = require$$0;
var describe = function() {
  return shape.kind + " " + shape.area(3) + " " + shape.default;
};
const items = data.items;
console.log("eval index");
console.log("greet " + greet$1("ada") + " " + shout("bob"));
console.log("cycle " + cycleAExports.report());
console.log("label " + label(data.name) + " total " + total(items));
console.log("barrel " + own + " " + twice(4) + " " + Object.keys(barrel).sort().join(","));
console.log("bridge " + describe());
const counter = new Counter();
counter.add(2);
console.log("counter " + counter.value + " " + Counter.name);
Promise.resolve().then(() => lazy$1).then((lazy2) => {
  console.log("lazy " + lazy2.value + " " + lazy2.default);
});
console.log("eval lazy");
const value = 42;
const lazy = "lazy-default";
const lazy$1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  default: lazy,
  value
}, Symbol.toStringTag, { value: "Module" }));
export {
  Counter,
  label,
  total
};
