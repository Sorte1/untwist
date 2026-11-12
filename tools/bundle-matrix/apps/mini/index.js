import { twice } from "./cjs.js";

console.log("eval index " + twice(21));
import("./lazy.js").then((lazy) => console.log("lazy " + lazy.value));
