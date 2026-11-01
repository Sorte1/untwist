function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
var c = b["k" + f];
if (c !== undefined) { return c; }
b["k" + f] = i;
return i;
}), b(c, d);
}
Object.defineProperty(Function.prototype, "k1", { get: function () { return "FP"; }, configurable: true });
console.log(b(1));
console.log(b(3));
