function a() { var h = ['A', 'x', 'y', '2A', '5', 'm5', 'm6', 'm7', '9', 'm9', 'm10', '@', '8', 'm13', '3']; a = function () { return h; }; return a(); }
function b(i, k) {
var e = a();
return (b = function (i, k) {
if (b.c === undefined) { b.c = {}; }
var n = i + e[0];
var v = b.c[n];
if (v === undefined) { v = e[i]; if (v === '@') { v = b(i + 1); } b.c[n] = v; }
return v;
}), b(i, k);
}
(function (c, d) {
var e = c();
while (!![]) {
try {
var f = parseInt(b(1)) + parseInt(b(11));
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 11);
console.log(b(1), b(4), b(5));
