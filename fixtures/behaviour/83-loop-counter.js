function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
b.n = (b.n || 0) + 1;
return b.n >= 3 ? "LATE" : i;
}), b(c, d);
}
for (var q = 0; q < 3; q++) {
console.log(b(1));
}
