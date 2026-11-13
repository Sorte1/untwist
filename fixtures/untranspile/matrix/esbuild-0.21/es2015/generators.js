var __knownSymbol = (name, symbol) => (symbol = Symbol[name]) ? symbol : Symbol.for("Symbol." + name);
var __typeError = (msg) => {
  throw TypeError(msg);
};
var __await = function(promise, isYieldStar) {
  this[0] = promise;
  this[1] = isYieldStar;
};
var __yieldStar = (value) => {
  var obj2 = value[__knownSymbol("asyncIterator")], isAwait = false, method, it2 = {};
  if (obj2 == null) {
    obj2 = value[__knownSymbol("iterator")]();
    method = (k) => it2[k] = (x) => obj2[k](x);
  } else {
    obj2 = obj2.call(value);
    method = (k) => it2[k] = (v) => {
      if (isAwait) {
        isAwait = false;
        if (k === "throw") throw v;
        return v;
      }
      isAwait = true;
      return {
        done: false,
        value: new __await(new Promise((resolve) => {
          var x = obj2[k](v);
          if (!(x instanceof Object)) __typeError("Object expected");
          resolve(x);
        }), 1)
      };
    };
  }
  return it2[__knownSymbol("iterator")] = () => it2, method("next"), "throw" in obj2 ? method("throw") : it2.throw = (x) => {
    throw x;
  }, "return" in obj2 && method("return"), it2;
};
function* range(start, end, step = 1) {
  for (let i = start; i < end; i += step) yield i;
}
function* inner() {
  const got = yield "first";
  yield "got " + got;
  return "inner-done";
}
function* outer() {
  const r = yield* __yieldStar(inner());
  yield r;
  try {
    yield "try";
  } finally {
    yield "cleanup";
  }
}
console.log([...range(0, 5)].join(","), Array.from(range(1, 10, 3)).join(","));
const it = outer();
console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
const obj = {
  *[Symbol.iterator]() {
    yield 1;
    yield 2;
  }
};
console.log([...obj].join(","));
function* thrower() {
  try {
    yield 1;
  } catch (e) {
    yield "caught " + e;
  }
}
const t = thrower();
t.next();
console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
function* fib() {
  let [a, b] = [0, 1];
  while (true) {
    yield a;
    [a, b] = [b, a + b];
  }
}
const f = fib();
const first = [];
for (const v of f) {
  if (v > 20) break;
  first.push(v);
}
console.log(first.join(","));
