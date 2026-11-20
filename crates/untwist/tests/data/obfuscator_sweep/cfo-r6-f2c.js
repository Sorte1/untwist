function f(setter) {
  const d = { abcde: "x", fghij: function (a, b) { return a === b; } };
  const v = { kkkkk: d.abcde, fffff: setter };
  v.fffff?.().x;
  console.log(d.fghij(v.kkkkk, "x") ? "then" : (d.abcde = "changed"));
  console.log(d.abcde);
}
f(function () { this.kkkkk = "y"; return {}; });
