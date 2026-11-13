"use strict";
function _defineProperty(obj, key, value) {
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
        _defineProperty(this, "count", 0);
        _defineProperty(this, "step", 1);
        _defineProperty(this, "id", ++seq);
        _defineProperty(this, "label", "c" + this.id);
        _defineProperty(this, _ref, "dynamic");
        _defineProperty(this, "inc", ()=>{
            this.count += this.step;
            return this;
        });
        if (step) this.step = step;
        Counter.instances++;
    }
}
_defineProperty(Counter, "instances", 0);
_defineProperty(Counter, "created", Counter.instances + 1);
const c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
const { inc  } = c;
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
        super(...args);
        _defineProperty(this, "value", 5);
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
        super(...args);
        _defineProperty(this, "x", 1);
    }
}
const df = new Defines();
console.log(df.x, df.setterRan === true);
