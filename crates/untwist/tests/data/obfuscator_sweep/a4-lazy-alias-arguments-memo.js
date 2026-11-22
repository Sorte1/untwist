var aQ = ["s0", "s1", "s2", "log"];
function bQ(n, t) {
  return bQ = function (t, r) {
    var e = aQ[t -= 0];
    if (void 0 === bQ.k) {
      bQ.h = function (i) { var s = "", j; for (j = 0; j < i.length; j++) s += i.charAt(j); return s; }, n = arguments, bQ.k = !0;
    }
    var d = t + aQ[0], o = n[d];
    return o ? e = o : (e = bQ.h(e), n[d] = e), e;
  }, bQ(n, t);
}
var z = bQ;
console.log(z(1));
console.log(bQ(1), z(2));
function f() { var y = bQ; return y(2) + y(0); }
console.log(f(), bQ(0));
