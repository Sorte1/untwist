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
function shadow(a) {
  return __async(this, null, function* () {
    let a = "inner";
    yield null;
    return a;
  });
}
var self = { v: "self" };
function receiver() {
  return __async(self, null, function* () {
    yield null;
    return this.v;
  });
}
function lengthy(_0, _1) {
  return __async(this, arguments, function* (a) {
    yield null;
    return a + arguments.length;
  });
}
log.push("lengths " + shadow.length + " " + receiver.length + " " + lengthy.length);
shadow("outer").then((x) => receiver().then((y) => lengthy("l", 2).then((z) => console.log(x, y, z, log.join(";")))));
