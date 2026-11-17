function f(a) { return a + 1; }
var F = Reflect.get((async function () {}), "constructor"); F("f = function () { return 99; }")().then(function () { console.log(f(1)); });
console.log(f(1), f(2));
