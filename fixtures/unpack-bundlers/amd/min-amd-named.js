define("lib/util", ["exports"], function (exports) {
  console.log("eval util");
  exports.twice = function (n) {
    return n * 2;
  };
});
define("lib/tag", ["./util"], function (util) {
  var label;
  console.log("eval tag");
  label = "tag" + util.twice(1);
  return label;
});
define("min-amd-named", ["lib/util", "lib/tag", "path"], function (util, tag, path) {
  console.log("eval entry", util.twice(4), tag, path.posix.join("a", "b"));
  return { done: true };
});
