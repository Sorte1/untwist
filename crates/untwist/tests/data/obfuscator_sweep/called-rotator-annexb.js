function outer() {
  var h = 1;
  {
    function arr() { var a = ["a", "b", "c"]; arr = function () { return a; }; return arr(); }
    var al = arr();
    function h() { return 2; }
    for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } }
    console.log(arr()[0]);
  }
  return typeof h;
}
console.log(outer());
