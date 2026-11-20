class A { tag() { return "A"; } }
class B extends A { m(arr) {
  var res = [];
  {
    var al = arr();
    var t = super.tag();
    for (;;) {
      try { res.push(t); if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "r " + res;
} }
console.log(new B().m(function () { return ["a", "b", "c"]; }));
