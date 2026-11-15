var arr = ["x", "y"]; function look(i) { return arr[i]; }
Reflect.set.call(null, globalThis, "arr", ["p", "q"]);
console.log(look(0) + look(1));
