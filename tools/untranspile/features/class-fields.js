let seq = 0;
class Counter {
  count = 0;
  step = 1;
  id = ++seq;
  label = "c" + this.id;
  static instances = 0;
  static created = Counter.instances + 1;
  ["dyn" + seq] = "dynamic";
  inc = () => {
    this.count += this.step;
    return this;
  };
  constructor(step) {
    if (step) this.step = step;
    Counter.instances++;
  }
}
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
  setup() {}
}
class Derived extends Base {
  value = 5;
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
  x = 1;
}
const df = new Defines();
console.log(df.x, df.setterRan === true);
