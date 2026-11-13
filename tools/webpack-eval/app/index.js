import { greet, total, Counter, makeAdders } from "./util.js";
import legacy from "./legacy.js";
import strictCjs from "./strict-cjs.js";
import data from "./data.json";
import * as shapes from "./shapes.js";

console.log(greet("webpack"), total([1, 2, 3, 4]));
const counter = new Counter(2);
counter.add(3);
console.log("counter", counter.value, Counter.name);
console.log("adders", makeAdders(3).map((add) => add(10)).join(","));
console.log("legacy", legacy.bump(), legacy.bump(), legacy.selfIsExports, legacy.blockFn());
console.log("strict cjs", strictCjs.mode(), strictCjs.label);
console.log("data", data.name, data.items.length);
console.log("shapes", Object.keys(shapes).sort().join(","), shapes.area(shapes.unit));
