function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { bbbbb: d.abcde };
  m.bbbbb = "y";
  m.aaaaa = m.bbbbb;
  const v = m;
  if (v.aaaaa === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
