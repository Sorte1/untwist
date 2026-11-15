(function () {
function hi() { var o; o = true; var r = (n, t) => { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; }; var s = r(this, function () { return s.toString().search("(((.+)+)+)+$").toString().constructor(s).search("(((.+)+)+)+$"); }); s(); o = true; var r2 = (n, t) => { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; }; var s2 = r2(this, function () { return s2.toString().search("(((.+)+)+)+$").toString().constructor(s2).search("(((.+)+)+)+$"); }); s2(); return 1; }
console.log("x", hi());
})();
