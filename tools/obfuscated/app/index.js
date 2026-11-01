import { add, scale, PRECISION } from "./math.js";
import Greeter, { shout } from "./strings.js";
import legacy from "./legacy.cjs";
import { createStore } from "./store.js";
import { table, pad } from "./format.js";
import Emitter from "./events.js";

const greeter = new Greeter("player");
console.log(greeter.greet());
console.log(shout("ready set go"));
console.log("sum", add(2, 3), "scaled", scale(7), "precision", PRECISION);

const store = createStore({ score: 0, lives: 3, name: "anon" });
const events = new Emitter();
events.on("hit", (points) => store.update("score", store.get("score") + points));
events.on("hit", () => console.log("hit registered"));
events.emit("hit", 25);
events.emit("hit", 40);
store.update("lives", store.get("lives") - 1);
console.log(table(store.snapshot()));
console.log(pad("left", 10) + "|" + pad("right", 10, true) + "|");
console.log(legacy.describe(), legacy.loader(), legacy.checksum("obfuscated webpack fixture"));
console.log("keys", Object.keys(store.snapshot()).join(","), "history", store.history().length);
