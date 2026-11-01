function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i.charAt(0) + i.slice(1);
}), b(c, d);
}
function def(O, k, v) { O[k] = v; }
var d2 = def;
d2(String.prototype, "charAt", function () { return "Z"; });
console.log(b(0));
