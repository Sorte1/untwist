function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde };
  m.kkkkk = "y";
  const v = m;
  if (m.kkkkk === "x") { console.log("live"); } else { d.abcde = "changed"; }
  console.log(d.abcde, d.fghij(1, 1), v.kkkkk);
}
f();
