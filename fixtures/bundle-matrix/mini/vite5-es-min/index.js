console.log("eval cjs");
var e = function(o) {
  return o * 2;
};
console.log("eval index " + e(21));
import("./lazy.chunk.js").then((o) => console.log("lazy " + o.value));
