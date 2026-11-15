(function () {
  var a = 1;
  var eval = globalThis.eval;
  eval("a = 'str'");
  console.log(typeof a, a);
})();
