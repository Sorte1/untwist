function F9() {}
F9.prototype = globalThis;
console.log(typeof new F9()[["_0", "x331b"].join("")]);
