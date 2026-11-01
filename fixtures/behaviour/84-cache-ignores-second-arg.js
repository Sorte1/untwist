function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (b.C === undefined) { b.C = {}; }
var hit = b.C[f];
if (hit !== undefined) { return hit; }
var i = e[f] + ":" + g;
b.C[f] = i;
return i;
}), b(c, d);
}
function X() { return b(1, "A"); }
var y = b(1, "B");
function Z() { return b(1, "A"); }
console.log(X());
console.log(y);
console.log(Z());
