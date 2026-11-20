function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde };
  m.kkkkk = "y";
  const v = m;
  console.log(v.kkkkk === "y" ? "alias" : "no");
  console.log(m.kkkkk === "x" ? "then" : (d.abcde = "changed"));
  console.log(d.abcde);
}
f();
