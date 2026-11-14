var a, b;
a || (a = 1, b = { v: 1 });
console.log("hasRequiredModule1" in globalThis);
