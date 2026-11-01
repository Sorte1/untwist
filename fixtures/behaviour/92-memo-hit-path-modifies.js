function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (b.C === undefined) { b.C = {}; }
var l = b.C[f];
if (l !== undefined) { return l + "?"; }
var i = e[f];
b.C[f] = i;
return i;
}), b(c, d);
}
function first() { return b(1); }
var mid = b(1);
function last() { return b(1); }
console.log(first());
console.log(mid);
console.log(last());
