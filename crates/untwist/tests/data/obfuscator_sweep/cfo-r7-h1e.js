function setter() { this.kkkkk = "y"; return 1; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  function call() { return m.fffff(); }
  const m = {};
  m.kkkkk = d.abcde;
  m.fffff = setter;
  m.ggggg = call();
  m.fffff = function () { return 1; };
  const v = m;
  call();
  if (v.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
