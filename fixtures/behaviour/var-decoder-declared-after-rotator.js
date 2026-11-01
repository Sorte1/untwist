function a() { var h = ['p', 'q', 'log', 'r', 's']; a = function () { return h; }; return a(); }
(function (c, d) {
var e = c();
var t = 0;
while (!![]) {
try {
if (b(0) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (t++ > 11) break;
}
})(a, 0);
var b = function (c, d) {
var e = a();
c = c - 0;
return e[c];
};
console.log(b(0), b(1), b(2), b(3), b(4));
