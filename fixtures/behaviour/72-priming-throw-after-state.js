function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
b.n = (b.n || 0) + 1;
return i.charAt(0) + b.n;
}), b(c, d);
}
console.log(b(1));
console.log(b(3));
