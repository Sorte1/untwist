function g() {
  if ("a" === "b") { var x = 1; } else { const x = function () {}; console.log(x.name); }
  if (true) { if ("a" === "b") { var y = 1; } else { const y = () => 0; console.log(y.name); } }
  return [typeof x, typeof y].join();
}
console.log(g());
