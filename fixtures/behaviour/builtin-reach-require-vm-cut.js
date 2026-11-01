
function a() { var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i + ([] instanceof Object ? '!' : '?');
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
require('vm').runInThisContext('Array.prototype.__proto__ = null');
console.log(b(0));
console.log(b(1));
console.log(b(3));
