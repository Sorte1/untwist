(function () {
  module.require("./patch.js");

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
