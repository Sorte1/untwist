let counter = 0;
class Fields {
  a = 1;
  b = this.a + 1;
  static s = ++counter;
  static t = Fields.s + 1;
  ["k" + counter] = "dyn";
  m() {
    return this.a + this.b;
  }
}
const f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
class Parent {
  constructor() {
    this.init();
  }
  init() {}
}
class Kid extends Parent {
  x = 1;
  init() {
    this.seen = this.x;
  }
}
console.log(new Kid().seen, new Kid().x);
class WithStaticBlock {
  static z;
  static {
    this.z = 42;
  }
}
console.log(WithStaticBlock.z);
class Arrowed {
  name = "arrowed";
  get = () => this.name;
}
const { get } = new Arrowed();
console.log(get());
