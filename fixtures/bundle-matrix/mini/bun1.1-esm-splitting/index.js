import {
  __require,
  __toESM
} from "./index-m70618hc.chunk.js";

// cjs.js
console.log("eval cjs");
var $twice = function(n) {
  return n * 2;
};

// index.js
console.log("eval index " + $twice(21));
import("./lazy-4bs0bwnr.chunk.js").then((lazy) => console.log("lazy " + lazy.value));
