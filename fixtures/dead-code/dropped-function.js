function probe() {
  var before = String(inner);
  if ("x" !== "x") {
    {
      function inner() {}
    }
  }
  return before;
}
console.log(probe());
