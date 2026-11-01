var key = "a";
function run() {
  var O = { a: 1, b: 2 };
  var read = O[key];
  return [read, O.b].join(",");
}
globalThis.key = "b";
console.log(run());
if (typeof require === "function" && typeof module === "object") {
  require("vm").runInThisContext(require("fs").readFileSync(__filename, "utf8"));
}
