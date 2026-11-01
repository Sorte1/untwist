function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
var box = [];
(function (c, d) { var e = c(); while (!![]) { try { if (b(0) === d) break; else e["push"](e["shift"]()); } catch (h) { e["push"](e["shift"]()); } } Object.assign(box, [1, 2]); })(a, "Hello World!");
console.log(box.length);
console.log(b(0));
console.log(b(1));
console.log(b(3));
