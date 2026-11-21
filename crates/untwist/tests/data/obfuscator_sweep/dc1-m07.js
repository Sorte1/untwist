globalThis.x = "G";
function g(c) {
  if (c) {
    if ("a" === "a") {
      if ("b" === "b") { let x = 1; console.log(x); } else { console.log(x); }
    } else { console.log(x + "dropped"); }
    console.log(x);
  }
  return x;
}
console.log(g(1));
