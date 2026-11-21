globalThis.x = "G";
function g(c) {
  while (c--) {
    if ("a" === "a") { let x = c; console.log(x); x = 5; }
    if ("b" === "b") { class x2 {} ; let y = x; console.log(y); }
    console.log(typeof y);
  }
  x = "W";
}
g(2); console.log(x);
