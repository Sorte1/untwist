function sloppy() {
  if (true) {
    "use strict";
    return this === undefined ? "strict" : "sloppy";
  }
}
function sloppyFirst() {
  if (1) "use strict";
  return this === undefined ? "strict" : "sloppy";
}
console.log(sloppy(), sloppyFirst());
