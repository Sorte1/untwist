globalThis.await = function (x) { return x; };
console.log(await (5), (function () { return typeof this; })());
