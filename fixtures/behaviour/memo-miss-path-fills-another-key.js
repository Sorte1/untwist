function a() { var h = ['x', '7', 'z', '@', 'OLD', 'NEW']; a = function () { return h; }; return a(); }
function b(i, k) {
var e = a();
return (b = function (i, k) {
if (b.c === undefined) { b.c = {}; }
var v = b.c[i];
if (v === undefined) { v = e[i]; if (v === '@') { v = b(i + 1); } b.c[i] = v; }
return v;
}), b(i, k);
}
(function (c, d) {
var e = c();
while (!![]) {
try {
var f = parseInt(e[0]) + (e[1] === 'z' ? 0 : parseInt(b(3)));
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 7);
console.log(b(0), b(4), b(1));
