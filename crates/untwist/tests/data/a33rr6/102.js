(function () {
  var t, e;
  t = () => 1; var n1 = t.name; t = class {}; var n2 = t.name; [t = function () {}] = []; var n3 = t.name; t = 0;
  function r() { if (t) return e; t = 1; e = { a: 1 }; return e; }
  console.log(n1, n2, n3, r().a);
})();
