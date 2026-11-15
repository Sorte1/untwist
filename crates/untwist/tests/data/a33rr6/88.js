(function () {
function hi() { var r = (function () { var o = true; return function (n, t) { var e = o ? function () { if (t) { var x = t.apply(n, arguments); t = null; return x; } } : function () {}; o = false; return e; }; })(); var s = r(this, function () { return s.toString().search("(((.+)+)+)+$").toString().constructor(s).search("(((.+)+)+)+$"); }); s(); console.log("x"); }
hi();
})();
