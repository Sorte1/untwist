function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
var k = 'x' + 1;
console.log(b(0x0) + b(0x1), window[k]);
