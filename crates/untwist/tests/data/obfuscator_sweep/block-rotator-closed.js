function outer(arr) {
  var al;
  {
    var w = arr();
    for (;;) {
      try { if (w[0] === "c") break; w.push(w.shift()); } catch (e) { w.push(w.shift()); }
    }
  }
  return "fell through";
}
console.log(outer(function () { return ["a", "b", "c"]; }));
