define("lib", [], function () {
  console.log("eval lib");
  return { value: 2 };
});
define(["lib"], function (lib) {
  console.log("eval entry", lib.value);
});
