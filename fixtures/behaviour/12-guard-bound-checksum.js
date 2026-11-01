function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
(function (c, d) { var e = c(); var tries = 0; while (!![]) { try { var s = b(0); if (s === d) break; else e["push"](e["shift"]()); } catch (h) { e["push"](e["shift"]()); } if (tries++ > 0) break; } })(a, "Hello World!");
console.log(b(0));
console.log(b(1));
console.log(b(3));
