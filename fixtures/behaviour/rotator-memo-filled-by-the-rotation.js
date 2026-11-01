function a() { var h = ["x","s1","5","s3","s4","1x","s6","s7","s8","s9","s10","s11","s12","s13","s14","s15","s16","s17","s18","s19","s20","s21","s22","s23"]; a = function () { return h; }; return a(); }
function b(i, k) {
  var e = a();
  return (b = function (i, k) {
    if (b.c === undefined) { b.c = {}; }
    var key = i + e[0];
    var v = b.c[key];
    if (v === undefined) { v = e[i]; b.c[key] = v; }
    return v;
  }), b(i, k);
}
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(e[0]) + parseInt(b(21));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 6);
console.log(b(2), b(3), b(4));
