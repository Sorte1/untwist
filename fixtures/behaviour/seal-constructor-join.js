function a() { var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i + ([1, 2].join('') === '12' ? '!' : '?');
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(0) === 'log!') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
var o = Object.seal(Array); o.prototype.join = function () { return 'X'; };
console.log(b(0));
console.log(b(1));
console.log(b(3));
