var shared = {
  get x() {
    shared = { x: "replaced", y: "replaced" };
    return "first";
  },
  y: "kept",
};
function fromShared() {
  var a = shared.x;
  var b = shared.y;
  console.log(a, b);
}
fromShared();
function fromTarget(e) {
  var a = e.x;
  var b = a.y;
  var c = a.z;
  console.log(b, c);
}
fromTarget({ x: { y: "inner y", z: "inner z" }, y: "outer y" });
function fromAliased(e) {
  var args = arguments;
  e.hook = function () {
    args[0] = { y: "swapped" };
  };
  var a = e.x;
  var b = e.y;
  console.log(a, b);
}
fromAliased({
  get x() {
    this.hook();
    return "x";
  },
  y: "y",
});
function fromConstant(e) {
  var a = e.x;
  var b = e["y"];
  let c = e.z;
  console.log(a, b, c);
}
fromConstant({ x: 1, y: 2, z: 3 });
