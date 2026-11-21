(function () {
  const g = typeof global === "object" ? global : this;
  g.setInterval(obfbn, 4e3);
})();
console.log("test");
function obfbn(d) {
  function h(j) {
    if (typeof j === "string") {
      return function (q) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + j / j).length !== 1 || j % 20 === 0) {
      var s;
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      var q;
      var r;
      var s;
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    h(++j);
  }
  try {
    if (d) {
      return h;
    } else {
      h(0);
    }
  } catch (j) {}
}
