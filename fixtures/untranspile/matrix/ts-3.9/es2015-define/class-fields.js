"use strict";
var _a;
let seq = 0;
class Counter {
    constructor(step) {
        Object.defineProperty(this, "count", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 0
        });
        Object.defineProperty(this, "step", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 1
        });
        Object.defineProperty(this, "id", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: ++seq
        });
        Object.defineProperty(this, "label", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: "c" + this.id
        });
        Object.defineProperty(this, _a, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: "dynamic"
        });
        Object.defineProperty(this, "inc", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: () => {
                this.count += this.step;
                return this;
            }
        });
        if (step)
            this.step = step;
        Counter.instances++;
    }
}
_a = "dyn" + seq;
Object.defineProperty(Counter, "instances", {
    enumerable: true,
    configurable: true,
    writable: true,
    value: 0
});
Object.defineProperty(Counter, "created", {
    enumerable: true,
    configurable: true,
    writable: true,
    value: Counter.instances + 1
});
const c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
const { inc } = c;
inc();
console.log(c.count, Object.keys(c).join(","));
class Base {
    constructor() {
        this.setup();
    }
    setup() { }
}
class Derived extends Base {
    constructor() {
        super(...arguments);
        Object.defineProperty(this, "value", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 5
        });
    }
    setup() {
        this.seen = this.value;
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
    constructor() {
        super(...arguments);
        Object.defineProperty(this, "x", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 1
        });
    }
}
const df = new Defines();
console.log(df.x, df.setterRan === true);
