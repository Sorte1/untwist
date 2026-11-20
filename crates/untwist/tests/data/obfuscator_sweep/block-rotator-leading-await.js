async function outer(arr, p) {
  var res = [];
  {
    var al = arr();
    var z = await p;
    for (;;) {
      try { res.push(z); if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "r " + res;
}
outer(function () { return ["a", "b", "c"]; }, Promise.resolve("p")).then(console.log);
