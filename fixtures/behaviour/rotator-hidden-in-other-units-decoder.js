function a() { var h = ['p', 'q', 'log', 'r', 's']; a = function () { return h; }; return a(); }
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
var t = 0;
while (!![]) {
try {
if (b(0) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (t++ > 50) break;
}
})(a, 0);
function m() { var h = ['u0', 'u1']; m = function () { return h; }; return m(); }
function n(i) {
var e = m();
(function (c, d) {
var e = c();
var t = 0;
while (!![]) {
try {
if (b(0) === 'r') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (t++ > 50) break;
}
})(a, 0);
return e[i];
}
console.log(b(0));
console.log(n(0));
console.log(b(0), b(1));
