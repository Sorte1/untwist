var __knownSymbol = (name, symbol) => (symbol = Symbol[name]) ? symbol : /* @__PURE__ */ Symbol.for("Symbol." + name);
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
  }, method = (k, call, wait, clear) => it[k] = (x) => (call = new Promise((yes, no, run) => (run = () => resume(k, x, yes, no), q ? q.then(run) : run())), clear = () => q === wait && (q = 0), q = wait = call.then(clear, clear), call), q, it = {};
  return generator = generator.apply(__this, __arguments), it[__knownSymbol("asyncIterator")] = () => it, method("next"), method("throw"), method("return"), it;
};
var __forAwait = (obj, it, method) => (it = obj[__knownSymbol("asyncIterator")]) ? it.call(obj) : (obj = obj[__knownSymbol("iterator")](), it = {}, method = (key, fn) => (fn = obj[key]) && (it[key] = (arg) => new Promise((yes, no, done) => (arg = fn.call(obj, arg), done = arg.done, Promise.resolve(arg.value).then((value) => yes({ value, done }), no)))), method("next"), method("return"), it);
function ticks(n) {
  return __asyncGenerator(this, null, function* () {
    for (let i = 0; i < n; i++) {
      yield new __await(null);
      yield i;
    }
  });
}
async function main() {
  const out = [];
  try {
    for (var iter = __forAwait(ticks(3)), more, temp, error; more = !(temp = await iter.next()).done; more = false) {
      const v = temp.value;
      out.push(v);
    }
  } catch (temp) {
    error = [temp];
  } finally {
    try {
      more && (temp = iter.return) && await temp.call(iter);
    } finally {
      if (error)
        throw error[0];
    }
  }
  console.log(out.join(","));
  const it = ticks(2);
  console.log(JSON.stringify(await it.next()), JSON.stringify(await it.return(9)), JSON.stringify(await it.next()));
  const fromSync = [];
  try {
    for (var iter2 = __forAwait([Promise.resolve("a"), "b"]), more2, temp2, error2; more2 = !(temp2 = await iter2.next()).done; more2 = false) {
      const v = temp2.value;
      fromSync.push(v);
    }
  } catch (temp2) {
    error2 = [temp2];
  } finally {
    try {
      more2 && (temp2 = iter2.return) && await temp2.call(iter2);
    } finally {
      if (error2)
        throw error2[0];
    }
  }
  console.log(fromSync.join(""));
}
main();
