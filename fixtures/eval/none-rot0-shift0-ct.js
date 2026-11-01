function a() { var h = ['log', 'Hello World!', 'console', 'deobfuscate']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
(function () { var t = { v: 0 }; return b(t.v); })();
(function () { var t = { v: 1 }; return b(t.v); })();
(function () { var t = { v: 2 }; return b(t.v); })();
(function () { var t = { v: 3 }; return b(t.v); })();
