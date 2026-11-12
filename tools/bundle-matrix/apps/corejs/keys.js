console.log("eval keys");
module.exports = function keys(value) {
  return Object.keys(value).map(function (key) {
    return key.replaceAll("a", "A");
  });
};
