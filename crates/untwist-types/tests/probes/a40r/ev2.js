(function (eval) {
  let a = 1;
  eval("a = 'str'");
  console.log(typeof a, a);
})(eval);
