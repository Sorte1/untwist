function setter() { this.kkkkk = "y"; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = { kkkkk: d.abcde, fffff: setter };
  m.ggggg = m.fffff();
  m.fffff = function () { return 1; };
  const v = m;
  if (v.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
