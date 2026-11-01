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
b(460);
b(461);
b(462);
b(463);
