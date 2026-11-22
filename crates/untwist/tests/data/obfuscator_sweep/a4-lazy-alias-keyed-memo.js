var aQ = ["s0", "s1", "s2", "log"];
function bQ(n, t) {
  return bQ = function (t, r) {
    var e = aQ[t -= 0];
    if (void 0 === bQ.k) {
      bQ.h = function (i, k) { return i + k; }, n = arguments, bQ.k = !0;
    }
    var d = t + aQ[0], o = n[d];
    return o ? e = o : (e = bQ.h(e, r), n[d] = e), e;
  }, bQ(n, t);
}
var z = bQ;
console.log(bQ(1, "a"));
console.log(z(1, "b"));
console.log(z(2, "c"), bQ(2, "d"));
