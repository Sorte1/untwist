(function () {
  function f(a = "d") {
    var a;
    const t = a;
    return typeof t;
  }
  function g(b) {
    var b;
    const u = b;
    return typeof u;
  }
  const h = function me() { var me; const v = me; return typeof v; };
  function k(c, d = () => c) { var c; const w = c; var c2 = 5; return typeof w; }
  const obj = { get x() { var arguments; const q = arguments; return typeof q; }, m() { var arguments; const q2 = arguments; return typeof q2; } };
  console.log(f(), g(1), h(), k(1), obj.x, obj.m());
})();
