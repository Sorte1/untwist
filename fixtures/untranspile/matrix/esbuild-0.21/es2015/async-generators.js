var __knownSymbol = (name, symbol) => (symbol = Symbol[name]) ? symbol : Symbol.for("Symbol." + name);
var __async = (__this, __arguments, generator) => {
  return new Promise((resolve, reject) => {
    var fulfilled = (value) => {
      try {
        step(generator.next(value));
      } catch (e) {
        reject(e);
      }
    };
    var rejected = (value) => {
      try {
        step(generator.throw(value));
      } catch (e) {
        reject(e);
      }
    };
    var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
    step((generator = generator.apply(__this, __arguments)).next());
  });
};
var __await = function(promise, isYieldStar) {
  this[0] = promise;
  this[1] = isYieldStar;
};
var __asyncGenerator = (__this, __arguments, generator) => {
  var resume = (k, v, yes, no) => {
    try {
      var x = generator[k](v), isAwait = (v = x.value) instanceof __await, done = x.done;
      Promise.resolve(isAwait ? v[0] : v).then((y) => isAwait ? resume(k === "return" ? k : "next", v[1] ? { done: y.done, value: y.value } : y, yes, no) : yes({ value: y, done })).catch((e) => resume("throw", e, yes, no));
    } catch (e) {
      no(e);
    }
  }, method = (k) => it[k] = (x) => new Promise((yes, no) => resume(k, x, yes, no)), it = {};
  return generator = generator.apply(__this, __arguments), it[__knownSymbol("asyncIterator")] = () => it, method("next"), method("throw"), method("return"), it;
};
var __forAwait = (obj, it, method) => (it = obj[__knownSymbol("asyncIterator")]) ? it.call(obj) : (obj = obj[__knownSymbol("iterator")](), it = {}, method = (key, fn) => (fn = obj[key]) && (it[key] = (arg) => new Promise((yes, no, done) => (arg = fn.call(obj, arg), done = arg.done, Promise.resolve(arg.value).then((value) => yes({ value, done }), no)))), method("next"), method("return"), it);
function ticker(n) {
  return __asyncGenerator(this, null, function* () {
    for (let i = 0; i < n; i++) {
      yield new __await(null);
      yield i * 10;
    }
  });
}
function relay(src) {
  return __asyncGenerator(this, null, function* () {
    try {
      for (var iter = __forAwait(src), more, temp, error; more = !(temp = yield new __await(iter.next())).done; more = false) {
        const v = temp.value;
        yield v + 1;
      }
    } catch (temp) {
      error = [temp];
    } finally {
      try {
        more && (temp = iter.return) && (yield new __await(temp.call(iter)));
      } finally {
        if (error)
          throw error[0];
      }
    }
  });
}
function main() {
  return __async(this, null, function* () {
    const seen = [];
    try {
      for (var iter = __forAwait(relay(ticker(3))), more, temp, error; more = !(temp = yield iter.next()).done; more = false) {
        const v = temp.value;
        seen.push(v);
      }
    } catch (temp) {
      error = [temp];
    } finally {
      try {
        more && (temp = iter.return) && (yield temp.call(iter));
      } finally {
        if (error)
          throw error[0];
      }
    }
    console.log(seen.join(","));
    const it = ticker(5);
    console.log(JSON.stringify([yield it.next(), yield it.return("x"), yield it.next()]));
    const mixed = [];
    try {
      for (var iter2 = __forAwait([Promise.resolve("a"), "b", later("c")]), more2, temp2, error2; more2 = !(temp2 = yield iter2.next()).done; more2 = false) {
        const v = temp2.value;
        mixed.push(v);
      }
    } catch (temp2) {
      error2 = [temp2];
    } finally {
      try {
        more2 && (temp2 = iter2.return) && (yield temp2.call(iter2));
      } finally {
        if (error2)
          throw error2[0];
      }
    }
    console.log(mixed.join(""));
  });
}
function later(v) {
  return new Promise((resolve) => setTimeout(() => resolve(v), 0));
}
main();
