function outer(arr) {
  {
    var al = arr();
    for (;;) {
      try { var n = al[0]; if (n === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "last " + n;
}
console.log(outer(function () { return ["a", "b", "c"]; }));
