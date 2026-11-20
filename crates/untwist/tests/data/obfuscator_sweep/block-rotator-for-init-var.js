function outer(arr) {
  {
    var al = arr();
    for (var i = 0; ; i++) {
      try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "turns " + i;
}
console.log(outer(function () { return ["a", "b", "c"]; }));
