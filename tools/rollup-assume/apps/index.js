import ms from "ms";
import assign from "object-assign";
import { counter, bump } from "./state.js";
import { hooked, configured } from "./host.js";
import legacy from "./legacy.cjs";
import host from "./host.cjs";

console.log("entry starts");
const here = new URL(import.meta.url);
console.log("scheme", here.protocol);
console.log("file", here.pathname.split("/").pop());
console.log("ms", ms(90000), ms("2h"));
console.log("assign", JSON.stringify(assign({ a: 1 }, { b: 2 })));
bump();
bump();
console.log("counter", counter);
console.log("legacy", legacy.twice(21), legacy.name);
console.log("config now", configured());
console.log("hook now", hooked(1));
console.log("host-cjs now", host.mode(), host.late(3));
setTimeout(() => {
  console.log("config later", configured());
  console.log("hook later", hooked(2));
  console.log("host-cjs later", host.mode(), host.late(4));
}, 0);
