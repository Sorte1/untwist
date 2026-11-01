function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (b.C === undefined) { b.C = {}; }
var hit = b.C[f];
if (hit !== undefined) { return hit; }
var i = e[f] + Object.keys(b.C).length;
b.C[f] = i;
return i;
}), b(c, d);
}
function later() { return b(1); }
var c0 = b(2);
console.log(later());
console.log(c0);
