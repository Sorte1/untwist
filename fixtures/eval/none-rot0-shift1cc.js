function a() { var h = ['log', 'Hello World!', 'console', 'deobfuscate']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 460;
var i = e[f];
return i;
}), b(c, d);
}
b(460);
b(461);
b(462);
b(463);
