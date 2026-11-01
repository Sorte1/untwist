function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
if (b.seen === undefined) { b.seen = true; return "FIRST"; }
return i;
}), b(c, d);
}
console.log(b(1));
console.log(b(3));
