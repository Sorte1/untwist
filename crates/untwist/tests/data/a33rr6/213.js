(function () {
function hi() { var o; o = true; var s = (0, function (n, t) { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; })(this, function () { return s.toString().search("(((.+)+)+)+$").toString().constructor(s).search("(((.+)+)+)+$"); }); s(); var k = [o]; console.log("x", k[0]); }
hi();
})();
