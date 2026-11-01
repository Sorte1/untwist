var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
var _a;
let counter = 0;
_a = "k" + counter;
const _Fields = class _Fields {
  constructor() {
    __publicField(this, "a", 1);
    __publicField(this, "b", this.a + 1);
    __publicField(this, _a, "dyn");
  }
  m() {
    return this.a + this.b;
  }
};
__publicField(_Fields, "s", ++counter);
__publicField(_Fields, "t", _Fields.s + 1);
let Fields = _Fields;
const f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
class Parent {
  constructor() {
    this.init();
  }
  init() {
  }
}
class Kid extends Parent {
  constructor() {
    super(...arguments);
    __publicField(this, "x", 1);
  }
  init() {
    this.seen = this.x;
  }
}
console.log(new Kid().seen, new Kid().x);
const _WithStaticBlock = class _WithStaticBlock {
};
__publicField(_WithStaticBlock, "z");
_WithStaticBlock.z = 42;
let WithStaticBlock = _WithStaticBlock;
console.log(WithStaticBlock.z);
class Arrowed {
  constructor() {
    __publicField(this, "name", "arrowed");
    __publicField(this, "get", () => this.name);
  }
}
const { get } = new Arrowed();
console.log(get());
