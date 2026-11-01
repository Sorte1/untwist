function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else obj[key] = value;
    return obj;
}
let counter = 0;
let _ref = "k" + counter;
class Fields {
    m() {
        return this.a + this.b;
    }
    constructor(){
        _define_property(this, "a", 1);
        _define_property(this, "b", this.a + 1);
        _define_property(this, _ref, "dyn");
    }
}
_define_property(Fields, "s", ++counter);
_define_property(Fields, "t", Fields.s + 1);
const f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
class Parent {
    init() {}
    constructor(){
        this.init();
    }
}
class Kid extends Parent {
    init() {
        this.seen = this.x;
    }
    constructor(...args){
        super(...args), _define_property(this, "x", 1);
    }
}
console.log(new Kid().seen, new Kid().x);
class WithStaticBlock {
}
_define_property(WithStaticBlock, "z", void 0);
WithStaticBlock.z = 42;
console.log(WithStaticBlock.z);
class Arrowed {
    constructor(){
        _define_property(this, "name", "arrowed");
        _define_property(this, "get", ()=>this.name);
    }
}
const { get } = new Arrowed();
console.log(get());
