function probe() {
  const before = typeof declared;
  if (false) {
    function declared() { return "dead1"; }
  }
  return before;
}
function probeTaken() {
  const before = typeof taken;
  if (true) {
    function taken() { return "taken"; }
  }
  return before + "," + typeof taken;
}
function probeElse() {
  if (true) {
    return typeof other;
  } else {
    function other() {}
  }
}
console.log(probe(), probeTaken(), probeElse());
