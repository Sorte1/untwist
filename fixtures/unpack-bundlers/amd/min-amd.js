define(["exports", "path"], function (exports, path) {
  "use strict";
  console.log("eval entry", path.posix.join("a", "b"));
  exports.value = 1;
});
