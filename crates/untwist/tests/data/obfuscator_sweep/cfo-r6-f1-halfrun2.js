function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: "y" };
  m.kkkkk = d.abcde;
  m.ggggg = (function () { if (m.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; } return 1; })();
  const v = m;
  if (v.kkkkk === "x") { console.log("alias"); }
  console.log(d.abcde);
}
f();
