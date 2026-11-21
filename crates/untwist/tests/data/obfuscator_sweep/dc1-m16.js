"use strict";
globalThis.x = "G";
function g(c) {
  if (c) {
    if ("a" === "a") { let x = 1; console.log(x); } else { function q() { return x; } console.log(q()); }
    if ("a" === "a") { function q2() { return x; } console.log(q2()); }
  }
}
g(1);
