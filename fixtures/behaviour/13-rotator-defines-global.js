function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
(function (c, d) { var e = c(); Object.defineProperty(globalThis, "flag", { value: "set", configurable: true }); while (!![]) { try { if (b(0) === d) break; else e["push"](e["shift"]()); } catch (h) { e["push"](e["shift"]()); } } })(a, "Hello World!");
console.log(typeof flag);
console.log(b(0));
console.log(b(1));
console.log(b(3));
