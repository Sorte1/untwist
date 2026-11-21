function setter() { this.kkkkk = "y"; return 1; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const m = {};
  m.kkkkk = d.abcde;
  m.fffff = setter;
  m.ggggg = m.fffff?.();
  m.fffff = function () { return 1; };
  const v = m;
  v.kkkkk === "x" ? console.log("then") : (d.abcde = "changed");
  console.log(d.abcde);
}
f();
