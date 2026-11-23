function g() {
  let y = 0;
  if (true) { let y = function () {}; const z = class {}; console.log(y.name, z.name); }
  let z = 1;
  return y + z;
}
console.log(g());
