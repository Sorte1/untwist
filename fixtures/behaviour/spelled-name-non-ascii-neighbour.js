function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i;
}), b(c, d);
}
var s = 'a¼';
globalThis[s.slice(0, 1)]().reverse();
console.log(b(0));
console.log(b(1));
console.log(b(3));
