globalThis.x = "G";
function g() {
  try { throw 1; } catch (e) {
    if ("a" === "a") { let x = 1; console.log(x); }
    console.log(typeof x, x);
  }
  for (let i = 0; i < 1; i++) {
    if ("a" === "a") { const x = 2; console.log(x); }
    console.log(typeof x, x);
  }
}
g();
