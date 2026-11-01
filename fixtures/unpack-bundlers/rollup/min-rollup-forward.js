(function () {
  "use strict";
  function log(line) {
    console.log(line);
  }
  const prefix = "[";
  function wrap(s) {
    return prefix + s + later();
  }
  log("eval wrap");
  function later() {
    return "]";
  }
  const suffix = "!";
  let hits = 0;
  function hit() {
    hits += 1;
    return hits;
  }
  log("eval hit");
  function kind(value) {
    return typeof value;
  }
  const joined = require("path").posix.join("a", "b");
  log("eval joined " + joined);
  log("entry " + wrap("x") + suffix + hit() + hit() + " " + kind(this));
})();
