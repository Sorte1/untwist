function probe() {
  if ("x" !== "x") {
    var hidden = 1;
  }
  return typeof hidden + ":" + String(hidden);
}
console.log(probe());
