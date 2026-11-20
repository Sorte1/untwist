function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde };
  m.kkkkk = "y";
  const v = m;
  if (m.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  if (v.kkkkk === "y") { console.log("alias"); }
  console.log(d.abcde);
}
f();
