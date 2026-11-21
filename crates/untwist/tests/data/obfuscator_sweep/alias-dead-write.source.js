function greet(name) {
  return "hello, " + name;
}
function shout(name) {
  return greet(name).toUpperCase() + "!";
}
console.log(greet("ada"));
console.log(shout("grace"));
