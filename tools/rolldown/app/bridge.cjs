console.log("eval bridge");
const shape = require("./shape.js");
exports.describe = function () {
  return shape.kind + " " + shape.area(3) + " " + shape.default;
};
