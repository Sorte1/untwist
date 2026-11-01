function a() { var h = ['x', '5', '3', 'y', '2']; a = function () { return h; }; return a(); }
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
var f = parseInt(e[0]) + parseInt(b(1));
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 8);
console.log(b(0), b(1), b(2), b(3), b(4));
