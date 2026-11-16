(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { a: 1 }; try { throw 7; } catch (e) { var e = 9; { function t() {} } } return e; }
  console.log(JSON.stringify(r()), typeof t);
})();
