"use strict";
var _a, _b;
let counter = 0;
class Fields {
    constructor() {
        this.a = 1;
        this.b = this.a + 1;
        this[_a] = "dyn";
    }
    m() {
        return this.a + this.b;
    }
}
_a = "k" + counter;
Fields.s = ++counter;
Fields.t = Fields.s + 1;
const f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
class Parent {
    constructor() {
        this.init();
    }
    init() { }
}
class Kid extends Parent {
    constructor() {
        super(...arguments);
        this.x = 1;
    }
    init() {
        this.seen = this.x;
    }
}
console.log(new Kid().seen, new Kid().x);
class WithStaticBlock {
}
_b = WithStaticBlock;
(() => {
    _b.z = 42;
})();
console.log(WithStaticBlock.z);
class Arrowed {
    constructor() {
        this.name = "arrowed";
        this.get = () => this.name;
    }
}
const { get } = new Arrowed();
console.log(get());
