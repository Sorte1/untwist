function f(a) { return a + 1; }
var s = Object.entries(Object.getOwnPropertyDescriptors(Reflect)).filter(function (e) { return e[0].length == 3 && e[0][0] == "s"; })[0][1].value; s(globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
