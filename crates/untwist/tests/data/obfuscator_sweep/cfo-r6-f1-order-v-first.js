function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde };
  m.kkkkk = "y";
  const v = m;
  if (v.kkkkk === "y") { console.log("alias"); }
  if (m.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
