(function () {
  function w(a) { return typeof a[1]; }
  console.log((function (n, o) { o = function () {}; n.x = o; return w(arguments); })({}, 5));
})();
