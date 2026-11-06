function inner() {
  const before = typeof late;
  if (false) {
    var late = 1;
    for (var i = 0; i < 2; i++) { var inLoop = i; }
  }
  return before + "," + typeof late + "," + typeof i + "," + typeof inLoop;
}
function elseBranch() {
  if (true) {
    var kept = "kept";
  } else {
    var gone = "dead1";
  }
  return typeof gone + "," + kept;
}
function whileHoist() {
  while (false) { var w = 1; }
  return typeof w;
}
function unused() {
  if (0) { var never = 1; }
  return "unused";
}
if (false) { var topLevel = 1; }
console.log(inner(), elseBranch(), whileHoist(), unused(), typeof topLevel, "topLevel" in globalThis);
