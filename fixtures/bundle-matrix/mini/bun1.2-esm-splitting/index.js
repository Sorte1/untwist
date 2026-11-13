import {
  __require,
  __toESM
} from "./index-qq0fcvx8.chunk.js";

// cjs.js
console.log("eval cjs");
var $twice = function(n) {
  return n * 2;
};

// index.js
console.log("eval index " + $twice(21));
import("./lazy-55j0kqam.chunk.js").then((lazy) => console.log("lazy " + lazy.value));
