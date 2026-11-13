"use strict";

class Resource {
  constructor(name) {
    this.name = name;
  }
}
class Pooled extends Resource {
  constructor(name) {
    var size = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 4;
    super(name);
    this.size = size;
    if (size > 8) {
      this.capped = true;
      return;
    }
    this.items = [];
  }
}
var first = new Pooled("a"),
  second = new Pooled("b", 16);
console.log(first.items.length, second.capped, second.items, first instanceof Resource);
