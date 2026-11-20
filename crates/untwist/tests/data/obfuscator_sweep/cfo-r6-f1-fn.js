function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.fghij };
  m.kkkkk = function (a, b) { return a !== b; };
  const v = m;
  if (m.kkkkk(1, 1)) { console.log("then"); } else { d.abcde = "changed"; }
  if (v.kkkkk(1, 2)) { console.log("alias"); }
  console.log(d.abcde);
}
f();
