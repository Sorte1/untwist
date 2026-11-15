(function () {
function hi() { var o; o = true; var r = (n, t) => { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; }; var s = r(this, function () { return s.toString().search("(((.+)+)+)+$").toString().constructor(s).search("(((.+)+)+)+$"); }); s(); return eval("o"); }
function hj() { var o; o = true; var r = (n, t) => { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; }; var s = r(this, function () { return s.toString().search("(((.+)+)+)+$").toString().constructor(s).search("(((.+)+)+)+$"); }); s(); return new Function("return typeof o")() + (0, eval)("typeof o"); }
console.log("x", hi(), hj());
})();
