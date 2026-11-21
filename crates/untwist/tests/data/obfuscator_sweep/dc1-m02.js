globalThis.x = "G";
function g() {
  if ("a" === "a") { let x = 1; console.log(x); }
  { console.log(typeof x); }
  if ("c" === "c") { console.log(x); }
}
g();
