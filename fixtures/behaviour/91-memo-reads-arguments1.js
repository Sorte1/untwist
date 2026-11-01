function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (b.C === undefined) { b.C = {}; }
var key = f + e[0], hit = b.C[key];
if (hit !== undefined) { return hit; }
var i = e[f] + ":" + arguments[1];
b.C[key] = i;
return i;
}), b(c, d);
}
function later() { return b(1, "A"); }
var y = b(1, "B");
console.log(later());
console.log(y);
console.log(later());
