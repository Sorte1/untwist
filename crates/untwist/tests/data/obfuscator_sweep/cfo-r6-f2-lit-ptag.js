function setter() { this.kkkkk = "y"; }
function f() {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const v = { kkkkk: d.abcde, fffff: setter };
  (v.fffff)`t`;
  if (v.kkkkk === "x") { console.log("then"); } else { d.abcde = "changed"; }
  console.log(d.abcde);
}
f();
