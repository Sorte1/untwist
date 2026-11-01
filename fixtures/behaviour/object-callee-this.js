function run(pick) {
  var O = { a: 1, b: 2 };
  var key = pick();
  O[key]().a = 5;
  return [O.a, O.b].join(",");
}
console.log(run(function () { return "valueOf"; }));
