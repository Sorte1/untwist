const log = require("./log");
const lib = require("./esm-lib.js");
log("lib " + typeof lib + " " + (typeof lib === "object" ? Object.keys(lib).sort().join(",") : lib));
function late() {
  return require("./lazy-inner").value;
}
log("late " + late());
