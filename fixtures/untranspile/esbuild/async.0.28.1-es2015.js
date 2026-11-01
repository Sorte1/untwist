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
const log = [];
const sleep = (v) => new Promise((resolve) => setTimeout(() => resolve(v), 0));
function one(x) {
  return __async(this, null, function* () {
    log.push("one-start " + x);
    const y = yield x;
    log.push("one-after " + y);
    return y * 2;
  });
}
function throws() {
  return __async(this, null, function* () {
    yield null;
    throw new RangeError("r");
  });
}
function withFinally() {
  return __async(this, null, function* () {
    try {
      yield throws();
    } catch (e) {
      log.push("caught " + e.constructor.name);
      return "ret";
    } finally {
      log.push("finally");
    }
  });
}
const obj = {
  base: 10,
  m(_0) {
    return __async(this, arguments, function* (a) {
      var _arguments = arguments;
      const f = () => __async(this, null, function* () {
        return this.base + a + _arguments.length;
      });
      return yield f();
    });
  }
};
function loop() {
  return __async(this, null, function* () {
    let s = 0;
    for (let i = 0; i < 3; i++) {
      s += yield sleep(i);
    }
    return s;
  });
}
function main() {
  return __async(this, null, function* () {
    const p = one(1);
    log.push("sync after call");
    console.log(yield p, yield withFinally(), yield obj.m(5), yield loop());
    console.log(one.name, typeof one);
    console.log(log.join("; "));
    const arrow = (n) => __async(null, null, function* () {
      return (yield sleep(n)) + 1;
    });
    console.log(yield arrow(1), yield Promise.all([one(2), arrow(3)]).then((v) => v.join(",")));
  });
}
main().catch((e) => console.log("fail", e.constructor.name));
