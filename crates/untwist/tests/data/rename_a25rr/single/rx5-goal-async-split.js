function f(a) { return a + 1; }
var AF = (async function () {}).constructor;
AF("a = [eval", "0][0]", "await null; return a")().then(function (run) { run("f = function () { return 99; }"); console.log(f(1), f(2)); });
