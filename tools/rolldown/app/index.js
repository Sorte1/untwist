import greet, { shout } from "./greet.cjs";
import data from "./data.json";
import { report } from "./cycle-a.cjs";
import { label, total, Counter } from "./util.js";
import * as barrel from "./barrel.js";
import { describe } from "./bridge.cjs";

const items = data.items;
console.log("eval index");
console.log("greet " + greet("ada") + " " + shout("bob"));
console.log("cycle " + report());
console.log("label " + label(data.name) + " total " + total(items));
console.log("barrel " + barrel.own + " " + barrel.twice(4) + " " + Object.keys(barrel).sort().join(","));
console.log("bridge " + describe());
const counter = new Counter();
counter.add(2);
console.log("counter " + counter.value + " " + Counter.name);
import("./lazy.js").then((lazy) => {
  console.log("lazy " + lazy.value + " " + lazy.default);
});
