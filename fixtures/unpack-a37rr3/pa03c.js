var f = "outer";
function t(){ { function f(){ return 1 } } return typeof f; }
globalThis.await = function (x) { return x; };
console.log(await (t()));
