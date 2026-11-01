var O = { a: 1, b: 2 };
var key = {
  toString: function () {
    if (globalThis.O) {
      globalThis.O.a = 7;
    }
    return "b";
  }
};
var read = O[key];
console.log([read, O.a, O.b].join(","));
if (typeof require === "function" && typeof module === "object") {
  require("vm").runInThisContext(require("fs").readFileSync(__filename, "utf8"));
}
