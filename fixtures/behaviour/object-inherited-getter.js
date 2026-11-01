Object.defineProperty(Object.prototype, "probe", {
  configurable: true,
  get: function () {
    this.a = 3;
    return 0;
  }
});
function run(key) {
  var O = { a: 1, b: 2 };
  var read = O[key];
  return [read, O.a, O.b].join(",");
}
console.log(run("probe"));
