var a, b;
a || (a = 1, b = { v: 1 });
console.log(globalThis.hasRequiredModule1);
