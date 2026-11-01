var inner = "outer";
function probe() {
  var before = String(inner);
  if ("x" === "x") {
    function inner() {
      return 1;
    }
  }
  return before + ":" + typeof inner;
}
console.log(probe());
