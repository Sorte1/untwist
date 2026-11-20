function outer(arr) {
  {
    var al = arr();
    for (;;) {
      try { function g() { return "g!"; } if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "fn " + typeof g;
}
console.log(outer(function () { return ["a", "b", "c"]; }));
