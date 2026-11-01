function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (f === -1) { return e; }
var i = e[f];
return i;
}), b(c, d);
}
console.log(b(0));
console.log(b(1));
console.log(b(2));
console.log(b(3));
