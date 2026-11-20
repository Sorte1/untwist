function setter() { this.kkkkk = "y"; return {}; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = {};
  m.kkkkk = d.abcde;
  m.fffff = setter;
  const v = m;
  v.fffff?.().x;
  if (v.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
