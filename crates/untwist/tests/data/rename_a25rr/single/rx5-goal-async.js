function f(a) { return a + 1; }
var AF = (async function () {}).constructor;
AF("await null; f = function () { return 99; }")().then(function () { console.log(f(1), f(2)); });
