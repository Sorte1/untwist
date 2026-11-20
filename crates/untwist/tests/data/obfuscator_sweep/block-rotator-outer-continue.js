function outer(arr) {
  var turns = 0;
  L: for (var k = 0; k < 3; k++) {
    {
      var al = arr();
      for (;;) {
        try { turns++; if (al[0] === "c") break; al.push(al.shift()); if (turns > 1) continue L; } catch (e) { al.push(al.shift()); }
      }
    }
  }
  return "turns " + turns;
}
console.log(outer(function () { return ["a", "b", "c"]; }));
