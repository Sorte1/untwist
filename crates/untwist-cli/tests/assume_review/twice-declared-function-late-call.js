(function () {
"use strict";
function dup() { return "first"; }
class Vector {
  constructor(x) { this.x = x; }
  add(other) { return new Vector(this.x + other.x); }
}
function length(v) { return Math.abs(v.x); }
var vectors = Object.freeze({ __proto__: null, Vector: Vector, length: length });
function dup() { return "second"; }
class Color {
  constructor(hex) { this.hex = hex; }
  mix(other) { return new Color(this.hex | other.hex); }
}
function parseColor(text) { return new Color(parseInt(text, 16)); }
var colors = Object.freeze({ __proto__: null, Color: Color, parseColor: parseColor });
const a = new Vector(1).add(new Vector(2));
console.log(length(a), parseColor("ff").mix(new Color(1)).hex);
console.log(length(new Vector(-3)), parseColor("10").hex);
console.log(Object.keys(vectors), Object.keys(colors));
console.log("late", dup());
})();
