function a() { var h = ['', 'x', 'y']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
if (b.init === undefined) { b.cache = arguments; b.init = true; }
var key = f + e[0], hit = b.cache[key];
if (hit === undefined) { hit = e[f] + '!'; b.cache[key] = hit; }
return hit;
}), b(c, d);
}
console.log(b(2));
console.log(b(0));
console.log(b(1));
