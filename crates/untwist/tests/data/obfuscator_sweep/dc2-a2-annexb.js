function g() {
  var x = 1;
  if ("a" === "a") { const x = 4; if (x) { function x() {} } console.log(x); }
  return typeof x;
}
console.log(g());
