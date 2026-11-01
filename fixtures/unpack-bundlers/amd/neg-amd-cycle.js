define("neg-amd-cycle/a", ["./b"], function (b) {
  console.log("eval a", typeof b);
  return { a: 1 };
});
define("neg-amd-cycle/b", ["./a"], function (a) {
  console.log("eval b", typeof a);
  return { b: 2 };
});
define("neg-amd-cycle", ["neg-amd-cycle/a"], function (a) {
  console.log("eval entry", typeof a);
});
