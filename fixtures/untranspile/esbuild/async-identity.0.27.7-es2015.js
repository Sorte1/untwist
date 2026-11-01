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
function one(x) {
  return __async(this, null, function* () {
    return x;
  });
}
console.log(one.length, Object.getPrototypeOf(one) === Function.prototype, typeof one.prototype);
const order = [];
function a1() {
  return __async(this, null, function* () {
    order.push(1);
    yield void 0;
    order.push(4);
    yield void 0;
    order.push(6);
  });
}
a1();
order.push(2);
Promise.resolve().then(() => order.push(5)).then(() => order.push(7)).then(() => order.push(8));
order.push(3);
setTimeout(() => console.log(order.join(",")), 0);
function thenable() {
  return __async(this, null, function* () {
    return { then(resolve) {
      resolve("thenable");
    } };
  });
}
thenable().then((v) => console.log(v));
