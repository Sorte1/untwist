var probe = "set";
console.log(typeof globalThis.probe === "string" ? "global" : "local");
if (typeof require === "function") {
  require("vm").runInThisContext(require("fs").readFileSync(__filename, "utf8"));
}
