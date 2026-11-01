function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i.charAt(0) + i.slice(1);
}), b(c, d);
}
function def(O, k, v) { arguments[1] = "charAt"; O[k] = v; }
def(String.prototype, "at", function () { return "Z"; });
console.log(b(0));
