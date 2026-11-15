(function () {
function hi() { var o; o = true; (function (n, t) { const e = o ? function () { let e; if (t) { e = t.apply(n, arguments); t = null; return e; } } : function () {}; o = false; return e; })(this, function () { return this.toString().search("(((.+)+)+)+$").toString().constructor(this).search("(((.+)+)+)+$"); })(); console.log("x", o); }
hi();
})();
