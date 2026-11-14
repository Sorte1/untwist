function _hasx(){}
var a, b;
a || (a = 1, b = { v: 1 });
var r = _has(this, "a");
console.log(typeof r === "object" ? JSON.stringify(r) : r);
