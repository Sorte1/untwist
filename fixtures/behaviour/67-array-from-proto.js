function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return Array.from([i]).join("");
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
Array.from = function () { return ["F"]; };
console.log(b(1));
console.log(b(3));
