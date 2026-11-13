import { twice } from "./cjs.js";
import { shared } from "./shared.js";
import { pad } from "tinylib";

console.log("eval index " + twice(21) + " " + shared("index") + " " + pad("x"));
async function main() {
  const lazy = await import("./lazy.js");
  console.log("lazy " + lazy.value + " " + lazy.nested());
  const legacy = await import("./legacy.js");
  console.log("legacy " + legacy.default.kind + " " + typeof legacy.default.run);
  const deep = await lazy.deeper();
  console.log("deeper " + deep.depth);
}
main().then(() => console.log("done"));
