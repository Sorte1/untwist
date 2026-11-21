globalThis.x = "G";
function g(c) {
  if (c) {
    if ("a" === "a") { let x = 1; console.log(x); } else { var h = function () { return x; }; }
    return function k(p = x) { return p; };
  }
}
console.log(g(1)());
