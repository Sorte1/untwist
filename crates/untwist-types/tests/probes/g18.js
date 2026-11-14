(function () {
  var f = 1;
  {
    function f() {}
  }
  console.log(typeof f);
  var k = "s";
  if (typeof f === "function") {
    function k() {}
  }
  console.log(typeof k);
})();
