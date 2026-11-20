function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde };
  m.ggggg = (function () { if (m.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; } return 1; })();
  m.kkkkk = "y";
  const v = m;
  if (v.kkkkk === "y") { console.log("alias"); }
  console.log(d.abcde);
}
f();
