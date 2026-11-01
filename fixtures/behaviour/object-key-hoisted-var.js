function run() {
  var O = { a: 1, b: 2 };
  var read = O[key];
  var key = "a";
  return [String(read), O.b].join(",");
}
console.log(run());
