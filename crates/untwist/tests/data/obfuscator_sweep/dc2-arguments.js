function g() {
  if (true) { let arguments = 5; console.log(arguments); }
  return arguments.length;
}
console.log(g(1, 2));
function h() {
  let arguments = 7;
  if (true) { let arguments = 5; console.log(arguments); }
  return arguments;
}
console.log(h(1, 2));
