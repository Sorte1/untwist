import data from "./data.json";
import { describe } from "./describe.js";

console.log("eval index");
console.log("at " + data.items.at(-1) + " hasOwn " + Object.hasOwn(data, "name"));
console.log("replaceAll " + "a-b-c".replaceAll("-", "+"));
console.log("describe " + describe(data));
