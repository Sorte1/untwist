function setter() { this.kkkkk = "y"; return 1; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = {};
  m.kkkkk = d.abcde;
  m.fffff = setter;
  m.ggggg = m.fffff();
  m.fffff = function () { return this; };
  const v = m;
  if (v.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
