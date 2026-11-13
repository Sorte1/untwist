// cjs.js
console.log("eval cjs");
var $twice = function(n) {
  return n * 2;
};

// index.js
console.log("eval index " + $twice(21));
import("./lazy-pv89dn65.chunk.js").then((lazy) => console.log("lazy " + lazy.value));
