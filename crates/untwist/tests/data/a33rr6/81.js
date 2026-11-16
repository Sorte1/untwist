(function () {
function hi() { var o, r = (o = true, (n, t) => { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; }), s = r(this, function () { return s.toString().search("(((.+)+)+)+$").toString().constructor(s).search("(((.+)+)+)+$"); }); s(); var u = r(this, () => { console.log("u ran"); return 3; }); console.log("x", u()); }
hi();
})();
