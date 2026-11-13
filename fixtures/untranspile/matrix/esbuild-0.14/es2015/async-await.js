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
const later = (v) => new Promise((resolve) => setTimeout(() => resolve(v), 0));
function fetchTwice(x) {
  return __async(this, null, function* () {
    log.push("start " + x);
    const a = yield later(x);
    const b = yield later(a + 1);
    return a + b;
  });
}
function guarded() {
  return __async(this, null, function* () {
    try {
      yield later(0);
      throw new TypeError("t");
    } catch (e) {
      log.push("caught " + e.name);
      return "r";
    } finally {
      log.push("finally");
    }
  });
}
const service = {
  base: 100,
  total(_0) {
    return __async(this, arguments, function* (n) {
      var _arguments = arguments;
      const add = (k) => __async(this, null, function* () {
        return this.base + k + _arguments.length;
      });
      let sum = 0;
      for (let i = 0; i < n; i++)
        sum += yield add(i);
      return sum;
    });
  }
};
class Loader {
  constructor() {
    this.items = [];
  }
  load(v) {
    return __async(this, null, function* () {
      yield null;
      this.items.push(v);
      return this.items.length;
    });
  }
}
function main() {
  return __async(this, null, function* () {
    const p = fetchTwice(1);
    log.push("after call");
    console.log(yield p, yield guarded(), yield service.total(3));
    const l = new Loader();
    console.log(yield l.load("a"), yield l.load("b"));
    const arrow = (n) => __async(this, null, function* () {
      return (yield later(n)) * 2;
    });
    console.log(yield arrow(4), (yield Promise.all([arrow(1), fetchTwice(2)])).join(","));
    console.log(log.join("; "));
  });
}
main().catch((e) => console.log("fail", e));
