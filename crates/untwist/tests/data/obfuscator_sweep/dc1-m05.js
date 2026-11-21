globalThis.x = "G";
function g() {
  var f = () => typeof x + x;
  if ("a" === "a") { let x = 1; console.log(x); }
  return f();
}
console.log(g());
