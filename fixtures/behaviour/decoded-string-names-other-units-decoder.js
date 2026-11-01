function a() { var h = ['xq', 'xx']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); return e[i].slice(1); }
function p() { var h = ['u0', 'u1']; p = function () { return h; }; return p(); }
function q(i) { var e = p(); return e[i]; }
Function(b(0) + ' = function (i) { return "EVIL" + i; }')();
console.log(q(0), q(1), b(1));
