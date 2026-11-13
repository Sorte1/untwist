"use strict";

var counter = {
  count: 0,
  start() {
    [1, 2, 3].forEach(n => {
      this.count += n;
    });
    return this.count;
  },
  nested() {
    var outer = () => {
      var inner = () => this.count;
      return inner();
    };
    return outer();
  },
  argsLater() {
    var f = () => arguments[0] + arguments.length;
    return f("x", "y");
  }
};
console.log(counter.start(), counter.nested(), counter.argsLater("a", "b"));
function Timer() {
  this.ticks = 0;
  var tick = () => ++this.ticks;
  tick();
  tick();
}
console.log(new Timer().ticks);
class Widget {
  constructor() {
    this.name = "w";
    this.handler = () => this.name;
  }
  render() {
    return [1].map(() => this.name + this.handler())[0];
  }
}
console.log(new Widget().render());
var id = x => x;
var obj = (a, b) => ({
  a,
  b
});
console.log(id(5), JSON.stringify(obj(1, 2)));
function outerFn() {
  return () => typeof new.target;
}
console.log(outerFn()(), new outerFn()());
