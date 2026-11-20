function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde };
  m.kkkkk = "y";
  const v = m;
  if (d.fghij(m.kkkkk, v.kkkkk)) { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
