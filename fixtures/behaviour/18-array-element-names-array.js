function a() { var h = ['a().reverse()', 'log', 'deobfuscate', 'console']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
eval(b(0));
console.log(b(0));
console.log(b(1));
console.log(b(3));
