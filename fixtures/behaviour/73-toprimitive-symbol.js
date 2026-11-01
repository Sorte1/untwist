function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
var o = { valueOf: function () { return i; } };
return "" + o;
}), b(c, d);
}
Object.prototype[Symbol.toPrimitive] = function () { return "T"; };
console.log(b(1));
console.log(b(3));
