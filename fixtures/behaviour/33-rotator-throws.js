function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (b['cache'] === undefined) { b['cache'] = {}; }
var hit = b['cache'][f];
if (hit !== undefined) { return hit; }
var i = e[f];
b['cache'][f] = i;
return i;
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
throw "stop";
})(a, 0);
console.log(b(0));
console.log(b(1));
console.log(b(3));
