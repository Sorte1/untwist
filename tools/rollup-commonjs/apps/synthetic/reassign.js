require("./log")("reassign eval");
module.exports = function shout(text) {
  return text + "!";
};
module.exports.extra = 7;
