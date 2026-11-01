function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return i.charAt(0) + i.slice(1);
}), b(c, d);
}
var def = function (O, k, v) { O.x = v; };
def = function (O, k, v) { O[k] = v; };
def(String.prototype, "charAt", function () { return "Z"; });
console.log(b(0));
