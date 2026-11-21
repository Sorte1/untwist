globalThis.x = "G";
function g() {
  if ("a" === "a") { let x = 1; console.log(x); }
  return typeof x + x;
}
console.log(g());
