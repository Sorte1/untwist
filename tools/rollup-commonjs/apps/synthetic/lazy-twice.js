var calls = 0;
module.exports = function () {
  calls += 1;
  return calls;
};
require("./log")("lazy-twice eval");
