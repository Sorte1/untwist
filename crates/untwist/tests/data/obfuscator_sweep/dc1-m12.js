globalThis.x = "G";
function g(c) {
  if (c) {
    if ("a" === "b") { console.log(x); } else { console.log(typeof x, x); }
    if ("a" === "a") { let x = 1; console.log(x); } else { console.log(x); }
  }
}
g(1);
