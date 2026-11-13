console.log("eval greet");
const path = require("node:path");
function greet(name) {
  return "hello " + path.basename(name);
}
greet.shout = function (name) {
  return greet(name).toUpperCase();
};
module.exports = greet;
module.exports.shout = greet.shout;
