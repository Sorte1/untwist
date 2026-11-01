function a() { var h = ['log', 'Hello World!', 'console', 'deobfuscate']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 460;
var i = e[f];
return i;
}), b(c, d);
}
(function () { var t = { v: 460 }; return b(t.v); })();
(function () { var t = { v: 461 }; return b(t.v); })();
(function () { var t = { v: 462 }; return b(t.v); })();
(function () { var t = { v: 463 }; return b(t.v); })();
