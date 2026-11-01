function modes(mode) {
  var MODES = { POINTS: 0, LINES: 1, TRIANGLES: 4 };
  var line = MODES.LINES;
  var named = MODES[mode];
  return [line, named, MODES.TRIANGLES].join(",");
}
function constantKey() {
  const key = "b";
  var TABLE = { a: "first", b: "second", 3: "third" };
  const index = 3;
  return [TABLE[key], TABLE[index], TABLE.a].join(",");
}
function constantAndUnknown(pick) {
  const key = "a";
  var MIXED = { a: 10, b: 20 };
  var picked = MIXED[pick];
  return [MIXED[key], picked, MIXED.b].join(",");
}
function keyAsArgument() {
  const key = "a";
  var ARGS = { a: "x", b: "y" };
  return String(ARGS[key]) + ARGS.b;
}
console.log(modes("TRIANGLES"), modes("missing"));
console.log(constantKey());
console.log(constantAndUnknown("b"), constantAndUnknown("toString") === undefined);
console.log(keyAsArgument());
