function c() { var h = ['log', 'Hello World!', 'patched']; c = function () { return h; }; return c(); }
function d(e, f) { var g = c(); d = function (i, j) { i = i - 0x0; return g[i]; }; return d(e, f); }
console[d(0x0)](d(0x1), typeof this.d);
