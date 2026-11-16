(function () {
  function w() { return typeof w.caller.arguments[1]; }
  console.log((function (n, o) { o = function () {}; n.x = o; return w(); })({}, 5));
})();
