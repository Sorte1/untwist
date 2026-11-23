function g() {
  if (true) { if (true) { let x = 4; console.log(x); } { function x() {} } }
  return typeof x;
}
console.log(g());
