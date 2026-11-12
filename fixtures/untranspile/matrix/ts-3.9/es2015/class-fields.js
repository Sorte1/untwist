"use strict";
var _a;
let seq = 0;
class Counter {
    constructor(step) {
        this.count = 0;
        this.step = 1;
        this.id = ++seq;
        this.label = "c" + this.id;
        this[_a] = "dynamic";
        this.inc = () => {
            this.count += this.step;
            return this;
        };
        if (step)
            this.step = step;
        Counter.instances++;
    }
}
_a = "dyn" + seq;
Counter.instances = 0;
Counter.created = Counter.instances + 1;
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
        this.value = 5;
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
        this.x = 1;
    }
}
const df = new Defines();
console.log(df.x, df.setterRan === true);
