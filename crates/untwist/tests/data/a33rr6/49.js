(function () {
  var t, e;
  function r() { if (t) return e; t = 1; [e = function () {}] = []; return e; }
  var t2, e2;
  function r2() { if (t2) return e2; t2 = 1; ({ a: e2 = class {} } = {}); return e2; }
  var t3, e3;
  function r3() { if (t3) return e3; t3 = 1; for (e3 of [0]) {} e3 ||= () => 1; return e3; }
  var t4, e4 = {};
  function r4() { if (t4) return e4; t4 = 1; var x = e4; e4 &&= function () {}; return e4; }
  console.log([r(), r2(), r3(), r4()].map(function (f) { return Error.prototype.toString.call(f); }).join());
})();
