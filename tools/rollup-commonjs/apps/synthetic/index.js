const log = require("./log");
log("index start");
const a = require("./cycle-a");
log("a.fromB " + a.fromB);
const shout = require("./reassign");
log("reassign " + shout("x") + " " + shout.extra);
const props = require("./props");
log("props " + props.one + props.two());
log("this " + require("./this-top").same);
function later() {
  return require("./lazy-inner").value;
}
log("before lazy");
log("lazy " + later() + " " + later());
function counter() {
  return require("./lazy-twice")();
}
log("twice " + counter() + counter());
const name = "./dyn-" + "target";
try {
  log("dynamic " + require(name).v);
} catch (error) {
  log("dynamic threw");
}
log("index end");
