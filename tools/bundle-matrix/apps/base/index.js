import chunk from "lodash/chunk";
import data from "./data.json";
import { report } from "./cycle-a.js";
import { label, total } from "./util.js";

const items = data.items;
console.log("eval index");
console.log("last " + items[items.length - 1] + " own " + Object.prototype.hasOwnProperty.call(data, "name"));
console.log("replace " + "a-b-c".split("-").join("+"));
console.log("chunk " + JSON.stringify(chunk(items, 2)));
console.log("cycle " + report());
console.log("label " + label(data.name) + " total " + total(items));
import("./lazy.js").then((lazy) => {
  console.log("lazy " + lazy.value + " " + lazy.default);
  return Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]);
}).then((settled) => console.log("settled " + settled.map((s) => s.status).join(",")));
