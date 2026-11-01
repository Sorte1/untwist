function a() { var h = ['A', 'x', '12', '2A', '3', '8', 'm6', 'm7', 'm8', 'm9', 'm10', 'm11', '8']; a = function () { return h; }; return a(); }
function b(i, k) {
var e = a();
return (b = function (i, k) {
if (b.c === undefined) { b.c = {}; }
var n = i + e[0];
if (b.c[n] === undefined) { b.c[n] = e[i]; }
return b.c[n];
}), b(i, k);
}
(function (c, d) {
var e = c();
while (!![]) {
try {
var f = parseInt(b(1)) + (b(2) === '12' ? parseInt(b(12)) : 0);
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 8);
console.log(b(1), b(4), b(5));
