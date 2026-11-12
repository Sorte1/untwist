"use strict";
function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else {
        obj[key] = value;
    }
    return obj;
}
let seq = 0;
let _ref = "dyn" + seq;
class Counter {
    constructor(step){
        _define_property(this, "count", 0);
        _define_property(this, "step", 1);
        _define_property(this, "id", ++seq);
        _define_property(this, "label", "c" + this.id);
        _define_property(this, _ref, "dynamic");
        _define_property(this, "inc", ()=>{
            this.count += this.step;
            return this;
        });
        if (step) this.step = step;
        Counter.instances++;
    }
}
_define_property(Counter, "instances", 0);
_define_property(Counter, "created", Counter.instances + 1);
const c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
const { inc } = c;
inc();
console.log(c.count, Object.keys(c).join(","));
class Base {
    setup() {}
    constructor(){
        this.setup();
    }
}
class Derived extends Base {
    setup() {
        this.seen = this.value;
    }
    constructor(...args){
        super(...args), _define_property(this, "value", 5);
    }
}
const dd = new Derived();
console.log(dd.seen, dd.value);
class Shadow {
    set x(v) {
        this.setterRan = true;
    }
}
class Defines extends Shadow {
    constructor(...args){
        super(...args), _define_property(this, "x", 1);
    }
}
const df = new Defines();
console.log(df.x, df.setterRan === true);
