function a() { var h = ['x', '4', 'z', 'q', 'OLD', 'NEW']; a = function () { return h; }; return a(); }
function b(i, k) {
var e = a();
return (b = function (i, k) {
if (b.c === undefined) { b.c = {}; }
if (b.c[i] === undefined) { b.c[i] = e[i]; }
return b.c[i];
}), b(i, k);
}
(function (c, d) {
var e = c();
while (!![]) {
try {
var f = parseInt(e[0]) + (e[1] === 'z' ? 0 : (b(1) === '4' ? parseInt(b(4)) : 0));
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 4);
console.log(b(0), b(4), b(3));
