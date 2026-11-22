function g() {
  let x = 0;
  if (true) { let x; x = function () {}; console.log(x.name); }
  return x;
}
console.log(g());
