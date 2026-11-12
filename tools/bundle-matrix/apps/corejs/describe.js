var keys = require("./keys.js");
console.log("eval describe");
exports.describe = function (value) {
  return keys(value).join("|") + " " + Object.hasOwn(value, "items");
};
