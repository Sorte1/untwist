function outer(y) {
  if ("a" === "a") {
    let y = 1;
    console.log(({ abcde: function (a, b) { return a + b; } }).abcde(y, 1));
  }
  return y;
}
console.log(outer(10));
