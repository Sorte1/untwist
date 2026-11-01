function probe() {
  var seen = typeof later;
  var later = 1;
  return seen + ":" + later;
}
console.log(probe());
