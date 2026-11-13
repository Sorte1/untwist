console.log("host-cjs evaluated", typeof hostConfig);
exports.mode = function () {
  return typeof hostConfig !== "undefined" && hostConfig !== null ? hostConfig.mode : "none";
};
exports.late = function (n) {
  return typeof hostHook === "function" ? hostHook(n) : "none";
};
