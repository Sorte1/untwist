function setter() { this.kkkkk = "y"; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const v = { kkkkk: d.abcde, fffff: setter };
  (v?.fffff)();
  if (v.kkkkk === "x") { console.log("live"); } else { d.abcde = "changed"; }
  console.log(d.abcde, d.fghij(1, 1));
}
f();
