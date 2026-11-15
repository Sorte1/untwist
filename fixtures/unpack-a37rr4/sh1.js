class K {}
function helperA() { return 1; }
const a = helperA();
function helperB() { "use strict"; return 2; }
const b = helperB();
yy = 4;
console.log("entry", a, b, yy, (function(){ return typeof this })());
