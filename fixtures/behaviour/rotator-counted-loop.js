function a() { var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
(function (c, n) { var e = c(); for (var k = 0; k < n; k++) e['push'](e['shift']()); })(a, 3);
console.log(b(0));
console.log(b(1));
console.log(b(2));
console.log(b(3));
