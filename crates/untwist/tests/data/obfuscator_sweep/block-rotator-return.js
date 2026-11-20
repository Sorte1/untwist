function outer(arr) {
  {
    var al = arr();
    for (;;) {
      try { if (al[0] === "c") return "returned"; al.push(al.shift()); if (al.length > 9) break; } catch (e) { al.push(al.shift()); }
    }
  }
  return "fell through";
}
console.log(outer(function () { return ["a", "b", "c"]; }));
