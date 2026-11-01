function probe() {
  if ("x" === "x") {
    let shade = 1;
    shade++;
  }
  return eval("typeof shade");
}
console.log(probe());
