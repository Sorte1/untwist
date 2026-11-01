function run(key) {
  var O = { a: 1, b: 2 };
  O[key]``.a = 6;
  return [O.a, O.b].join(",");
}
console.log(run("valueOf"));
