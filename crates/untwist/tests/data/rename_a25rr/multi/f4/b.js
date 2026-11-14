var a, b;
a || (a = 1, b = { v: 1 });
console.log(keysOf(globalThis).indexOf("a") >= 0);
