class Counter {
  constructor(start = 0) {
    this.count = start;
  }
  stepper(by = 1, label = "n" + this.count) {
    return () => {
      this.count += by;
      return label + ":" + this.count;
    };
  }
}
function tally(seed = 10, extra) {
  return () => this.base + seed + (extra ?? 0) + arguments.length;
}
const counter = new Counter(2);
const step = counter.stepper();
console.log(step(), step(), counter.stepper(5, "x")(), Counter.prototype.stepper.length);
console.log(tally.call({ base: 1 })(), tally.call({ base: 1 }, 3, 4)(), tally.length);
