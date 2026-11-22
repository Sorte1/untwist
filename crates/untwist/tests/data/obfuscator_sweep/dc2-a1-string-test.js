function g() {
  let x = 0;
  if ("a" === "a") { class x {} console.log(x.name); let y = function () {}; const x2 = class {}; console.log(y.name, x2.name); }
  return x;
}
console.log(g());
