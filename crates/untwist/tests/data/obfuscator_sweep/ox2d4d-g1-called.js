function outer() {
  var out = [];
  for (var i = 0; i < 2; i++) {
    var al = arr();
    for (;;) { try { var h; out.push(typeof h); h = 1; if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } }
    function arr() { var a = ["a", "b", "c"]; arr = function () { return a; }; return arr(); }
  }
  return out.join();
}
console.log(outer());
