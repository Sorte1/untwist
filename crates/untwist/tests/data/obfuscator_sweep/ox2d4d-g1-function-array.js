function outer() {
  var out = [];
  for (var i = 0; i < 2; i++) {
    function a() { var x = ["a", "b", "c"]; a = function () { return x; }; return a(); }
    for (var w = a(); ;) { try { var h; out.push(typeof h); h = 1; if (w[0] === "c") break; w.push(w.shift()); } catch (e) { w.push(w.shift()); } }
    out.push(a()[0]);
  }
  return out.join();
}
console.log(outer());
