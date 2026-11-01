function probe() {
  if ("x" === "x") {
    let shade = 1;
    shade++;
  }
  return typeof shade;
}
console.log(probe());
