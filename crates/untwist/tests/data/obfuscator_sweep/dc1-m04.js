globalThis.x = "G";
function g(c) {
  switch (c) {
    case 1: if ("a" === "a") { let x = 1; console.log(x); } break;
    case 2: console.log(typeof x, x);
  }
}
g(1); g(2);
