var a1 = (0, function () {});
var a2 = 1 ? function () {} : 0;
var a3; a3 = (a1(), function () {});
var a4 = true && (() => {});
var a5 = !1 || class {};
a6 = !0 ? function () {} : 0;
this.a7 = (0, function () {});
module.exports.a8 = 1 ? function () {} : 0;
console.log([a1, a2, a3, a4, a5, a6, this.a7, module.exports.a8].map(function (f) { return JSON.stringify(f.name); }).join(' '));
