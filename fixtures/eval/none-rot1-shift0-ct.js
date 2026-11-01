function a() { var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(0) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
(function () { var t = { v: 0 }; return b(t.v); })();
(function () { var t = { v: 1 }; return b(t.v); })();
(function () { var t = { v: 2 }; return b(t.v); })();
(function () { var t = { v: 3 }; return b(t.v); })();
