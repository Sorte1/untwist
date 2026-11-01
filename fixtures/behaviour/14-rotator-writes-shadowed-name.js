function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
var seen = "outer";
(function (c, d) { var e = c(); function g(seen) { return seen; } while (!![]) { try { if (b(0) === d) break; else e["push"](e["shift"]()); } catch (h) { e["push"](e["shift"]()); } } seen = "inner"; })(a, "Hello World!");
console.log(seen);
console.log(b(0));
console.log(b(1));
console.log(b(3));
