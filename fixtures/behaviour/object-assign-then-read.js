function run(key) {
  var O = { a: 1, b: 2 };
  O[key] = 9;
  return [O.a, O.b].join(",");
}
console.log(run("a"));
