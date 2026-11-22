function g() {
  var x = 1;
  if (true) { if (true) { let x = 4; { function x() {} } console.log(x); } }
  return typeof x;
}
console.log(g());
