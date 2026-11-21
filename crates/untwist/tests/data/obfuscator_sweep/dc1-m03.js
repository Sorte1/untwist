globalThis.x = "G";
function g() { console.log(typeof x); if ("a" === "a") { let x = 1; console.log(x); } }
g();
