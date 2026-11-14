(function () {
"use strict";
class Vector {
  constructor(x) { this.x = x; }
  add(other) { return new Vector(this.x + other.x); }
}
function length(v) { return Math.abs(v.x); }
var vectors = Object.freeze({ __proto__: null, Vector: Vector, length: length });
class Store {
  constructor() { this.items = []; }
}
let value = null;
function peek() { return value; }
var stores = Object.freeze({ __proto__: null, Store: Store, peek: peek });
value = new Store();
console.log(peek() === null, peek() instanceof Store);
value = function () {};
console.log(peek().name, new Store().items.length, Object.keys(stores));
console.log(typeof peek(), Object.keys(vectors));
})();
