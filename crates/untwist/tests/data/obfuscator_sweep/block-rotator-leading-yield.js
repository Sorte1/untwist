function* outer(arr) {
  var res = [];
  {
    var al = arr();
    var z = yield "y";
    for (;;) {
      try { res.push(z); if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "r " + res;
}
var it = outer(function () { return ["a", "b", "c"]; });
it.next();
console.log(it.next("v").value);
