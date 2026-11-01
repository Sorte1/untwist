function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
b = function (f, g) {
if (b.init === undefined) { b.init = true; b.cache = arguments; }
var hit = b.cache[f];
if (hit !== undefined && hit !== f) { return hit + '!'; }
return e[f] + '!';
};
b(3);
return b(c, d);
}
console.log(b(1));
console.log(b(0));
