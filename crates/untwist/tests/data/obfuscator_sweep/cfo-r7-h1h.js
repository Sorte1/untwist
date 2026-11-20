function f(g) {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = {};
  m.kkkkk = d.abcde;
  m.fffff = g;
  m.ggggg = m.fffff();
  m.fffff = function () { return 1; };
  const v = m;
  if (v.kkkkk === "x") { console.log("then"); } else { console.log("else"); d.abcde = "changed"; }
  console.log(d.abcde, v.kkkkk);
}
f(function () { delete this.kkkkk; });
