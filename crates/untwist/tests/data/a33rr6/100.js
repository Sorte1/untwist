(function () {
  var t, e;
  function r() { if (t) return e; t = 1; ({ e = function () {} } = {}); return e; }
  console.log(r().name, Error.prototype.toString.call(r()));
})();
