require("core-js/actual/array/at");
require("core-js/actual/object/group-by");
console.log("at " + [1, 2, 3].at(-1));
var groups = Object.groupBy([1, 2, 3, 4], function (n) {
  return n % 2 ? "odd" : "even";
});
console.log("group " + groups.odd.join(",") + " " + groups.even.join(","));
