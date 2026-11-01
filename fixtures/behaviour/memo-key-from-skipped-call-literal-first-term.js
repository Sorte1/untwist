function a() { var h = ['A', 'x', '12', '2A', '3', '1', '5', 'm7', 'm8', '15', 'm10', 'm11', '8']; a = function () { return h; }; return a(); }
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
var f = parseInt(b(1)) + parseInt(b(parseInt(b(2))));
if (f === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 16);
console.log(b(1), b(4), b(5));
