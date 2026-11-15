var arr = ["x", "y"]; function look(i) { return arr[i]; }
var S = Reflect.set; S(globalThis, "look", function () { return "z"; });
console.log(look(0) + look(1));
