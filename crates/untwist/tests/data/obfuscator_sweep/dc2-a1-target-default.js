function g() {
  let x = 0, y = 1;
  if (true) { let x, y; [x = () => 0] = []; ({ y = function () {} } = {}); console.log(x.name, y.name); }
  return x + y;
}
console.log(g());
