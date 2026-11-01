import { say } from "./say.js";
say("eval reexport");
export { shout as loud } from "./greet.js";
export * from "./math.js";
