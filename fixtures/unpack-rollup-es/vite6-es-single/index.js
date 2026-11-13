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
  if (Object.prototype.hasOwnProperty.call(n, "__esModule")) return n;
  var f = n.default;
  if (typeof f == "function") {
    var a = function a2() {
      if (this instanceof a2) {
        return Reflect.construct(f, arguments, this.constructor);
      }
      return f.apply(this, arguments);
    };
    a.prototype = f.prototype;
  } else a = {};
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
var greet$1 = { exports: {} };
var hasRequiredGreet;
function requireGreet() {
  if (hasRequiredGreet) return greet$1.exports;
  hasRequiredGreet = 1;
  console.log("eval greet");
  const path = require$$0$1;
  function greet2(name2) {
    return "hello " + path.basename(name2);
  }
  greet2.shout = function(name2) {
    return greet2(name2).toUpperCase();
  };
  greet$1.exports = greet2;
  greet$1.exports.shout = greet2.shout;
  return greet$1.exports;
}
var greetExports = requireGreet();
const greet = /* @__PURE__ */ getDefaultExportFromCjs(greetExports);
const name = "fixture";
const items$1 = [1, 2, 3, 4, 5];
const data = {
  name,
  items: items$1
};
var cycleA = {};
var cycleB = {};
var hasRequiredCycleB;
function requireCycleB() {
  if (hasRequiredCycleB) return cycleB;
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
  if (hasRequiredCycleA) return cycleA;
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
var hasRequiredMath;
function requireMath() {
  if (hasRequiredMath) return math;
  hasRequiredMath = 1;
  console.log("eval math");
  math.twice = function(n) {
    return n * 2;
  };
  math.half = (n) => n / 2;
  return math;
}
var mathExports = requireMath();
const own = "own";
const barrel = /* @__PURE__ */ _mergeNamespaces({
  __proto__: null,
  own
}, [mathExports]);
var bridge = {};
console.log("eval shape");
const kind = "square";
function area(side) {
  return side * side;
}
const shape = "shape-default";
const shape$1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  area,
  default: shape,
  kind
}, Symbol.toStringTag, { value: "Module" }));
const require$$0 = /* @__PURE__ */ getAugmentedNamespace(shape$1);
var hasRequiredBridge;
function requireBridge() {
  if (hasRequiredBridge) return bridge;
  hasRequiredBridge = 1;
  console.log("eval bridge");
  const shape2 = require$$0;
  bridge.describe = function() {
    return shape2.kind + " " + shape2.area(3) + " " + shape2.default;
  };
  return bridge;
}
var bridgeExports = requireBridge();
const items = data.items;
console.log("eval index");
console.log("greet " + greet("ada") + " " + greetExports.shout("bob"));
console.log("cycle " + cycleAExports.report());
console.log("label " + label(data.name) + " total " + total(items));
console.log("barrel " + own + " " + mathExports.twice(4) + " " + Object.keys(barrel).sort().join(","));
console.log("bridge " + bridgeExports.describe());
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
