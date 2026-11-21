function outer(arr) {
  var out = [];
  for (var i = 0; i < 2; i++) {
    {
      var w = arr();
      for (;;) {
        try { var h; out.push(typeof h); h = 1; if (w[0] === "c") break; w.push(w.shift()); } catch (e) { w.push(w.shift()); }
      }
    }
  }
  return out.join();
}
console.log(outer(function () { return ["a", "b", "c"]; }));
