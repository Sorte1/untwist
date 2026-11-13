import {
__require
} from "./chunk-928dc32b1861f43c.chunk.js";

// cjs.js
console.log("eval cjs");
var $twice = function(n) {
  return n * 2;
};

// index.js
console.log("eval index " + $twice(21));
import("./lazy.js").then((lazy) => console.log("lazy " + lazy.value));
