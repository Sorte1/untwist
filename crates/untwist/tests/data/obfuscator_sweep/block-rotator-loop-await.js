async function outer(arr) {
  var seen = [];
  {
    var al = arr();
    for (;;) {
      try { seen.push(await al[0]); if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "seen " + seen;
}
outer(function () { return ["a", "b", "c"]; }).then(console.log);
