class Base {
  constructor(a, b) {
    this.a = a;
  }
  m() {
  }
}
class Child extends Base {
}
console.log(Base.length, Child.length, Object.getOwnPropertyDescriptor(Base, "prototype").writable);
console.log(Object.getOwnPropertyNames(Child).sort().join(","));
class Errorish extends Error {
  constructor(m) {
    super(m);
    this.name = "Errorish";
  }
}
const err = new Errorish("boom");
console.log(err instanceof Errorish, err instanceof Error, err.message, err.name);
class ArrLike extends Array {
}
const al = new ArrLike();
al.push(1);
console.log(al.length, al instanceof ArrLike);
console.log(typeof Base.prototype.m.prototype);
try {
  new Base.prototype.m();
  console.log("constructible");
} catch (e) {
  console.log(e.constructor.name);
}
