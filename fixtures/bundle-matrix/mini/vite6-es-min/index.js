var r = {}, o;
function n() {
  return o || (o = 1, console.log("eval cjs"), r.twice = function(e) {
    return e * 2;
  }), r;
}
var c = n();
console.log("eval index " + c.twice(21));
import("./lazy.chunk.js").then((e) => console.log("lazy " + e.value));
