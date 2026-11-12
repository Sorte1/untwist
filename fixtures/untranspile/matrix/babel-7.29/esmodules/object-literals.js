"use strict";

var k = "dyn";
var x = 1;
var o = {
  x,
  [k + "1"]: 2,
  method() {
    return "m" + this.x;
  },
  get g() {
    return "get";
  },
  ["t".concat(k)]: 3,
  __proto__: {
    inherited: true
  }
};
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
  hello() {
    return "base";
  }
};
var sub = {
  __proto__: sup,
  hello() {
    return super.hello() + "+sub";
  }
};
console.log(sub.hello());
