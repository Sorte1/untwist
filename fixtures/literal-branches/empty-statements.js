function sloppy() {
  ;
  "use strict";
  return this === undefined ? "strict" : "sloppy";
}
function plain() {
  ;;
  return 1;
}
;
if (false) {
  console.log("dead");
}
;
console.log(sloppy(), plain());
