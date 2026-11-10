var o = {
  abcde: async function (a, b) { return a + b; },
  fghij: function* (a, b) { return a + b; },
  klmno: async function (f, x) { return f(x); }
};
function id(v) { console.log("called", v); return v; }
function t(p, q) {
  var r1 = o.abcde(p, q);
  var r2 = o.fghij(p, q);
  var r3 = o.klmno(id, q);
  console.log(typeof r1, typeof r2, typeof r3, r1 instanceof Promise, typeof r2.next);
}
t(1, 2);
