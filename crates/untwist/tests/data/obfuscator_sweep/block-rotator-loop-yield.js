function* outer(arr) {
  {
    var al = arr();
    for (;;) {
      try { yield al[0]; if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
}
console.log([...outer(function () { return ["a", "b", "c"]; })].join());
