(function () {
  var t, { e = function () {} } = {}, q = e;
  function r() { if (t) return e; t = 1; e = { a: 1 }; return e; }
  console.log(q.name, r().a);
})();
