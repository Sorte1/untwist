function F9() {}
F9.prototype = globalThis;
console.log(typeof new F9()[["d", "ec"].join("")]);
