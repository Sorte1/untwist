function g() {
  let x = 0, y = 1;
  if (true) { const { x = function () {} } = {}; const [y = class {}] = []; console.log(x.name, y.name); }
  return x + y;
}
console.log(g());
