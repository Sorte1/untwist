function g() {
  if ("a" === "b") { var x = 1; console.log(x); } else { const x = function () {}; console.log(x.name); }
  if ("a" === "b") { var y = 1; console.log(y); } else { if (true) { let y = class {}; console.log(y.name); } }
  return 1;
}
console.log(g());
