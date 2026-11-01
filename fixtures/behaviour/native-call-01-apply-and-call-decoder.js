function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = parseInt.bind(null)(f - 0);
var i = e[f];
var codes = [];
for (var k = 0; k < i.length; k++) codes.push(''.charCodeAt.call(i, k));
return String.fromCharCode.apply(null, codes);
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(0) === 'Hello World!') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
console.log(b(1));
console.log(b(3));
