(function () {
  require("vm").runInThisContext("Math.random = () => \"r\"; parseInt = () => null");

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
