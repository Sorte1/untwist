function c() { var h = ['log', 'Hello World!', 'patched']; c = function () { return h; }; return c(); }
function d(e, f) { var g = c(); d = function (i, j) { i = i - 0x0; return g[i]; }; return d(e, f); }
var imports = { c: 1, 'd': 2, get b() { return 3; } };
console[d(0x0)](d(0x1), Object.keys(imports).length);
