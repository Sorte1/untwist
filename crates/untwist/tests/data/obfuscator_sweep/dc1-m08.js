globalThis.x = "G";
function g(c) {
  if (c) {
    if ("a" === "a") {
      if ("b" === "b") { let x = 1; console.log(x); }
    }
  } else { return x; }
  return (function () { return x; })();
}
console.log(g(1), g(0));
