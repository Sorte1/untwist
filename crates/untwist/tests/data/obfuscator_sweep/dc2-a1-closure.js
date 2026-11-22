function g() {
  let x = 0;
  if (true) { let x; (function () { x = class {}; })(); console.log(x.name); }
  return x;
}
console.log(g());
