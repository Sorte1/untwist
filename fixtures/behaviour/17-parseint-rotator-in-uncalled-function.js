function a() { var h = ['1', '2', '3', '4']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
function never() {
(function (c, d) {
var e = c();
while (!![]) {
try {
var g = parseInt(b(0));
if (g === d) break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
}
})(a, 3);
}
console.log(b(0));
