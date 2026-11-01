function a() { var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 460;
var i = e[f];
return i;
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(460) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
(function () { var t = { v: 460 }; return b(t.v); })();
(function () { var t = { v: 461 }; return b(t.v); })();
(function () { var t = { v: 462 }; return b(t.v); })();
(function () { var t = { v: 463 }; return b(t.v); })();
