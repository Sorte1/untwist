function f(a) { return a + 1; }
(function () { var w = globalThis; var R = w["Reflect"]; [R].map(function (x) { return x.get(x, "set"); })[0] (globalThis, "f", function () { return 99; }); })();
console.log(f(1), f(2));
