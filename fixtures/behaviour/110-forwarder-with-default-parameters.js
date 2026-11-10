var o = { abcde: function (a, b = 5) { return a + b; }, fghij: function (f, x = 7) { return f(x); } };
function id(v) { return v; }
function t(p, q) {
  console.log(o.abcde(p, q), o.abcde(1, undefined), o.abcde(1, 2), o.fghij(id, q), o.fghij(id, undefined));
}
t(1, undefined);
