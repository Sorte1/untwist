var shared = { value: 7 };
define([], function () {
  console.log("eval entry", shared.value);
});
