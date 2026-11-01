function run(key) {
  var O = { a: 1, b: 2 };
  var kept = O.b;
  delete O[key];
  return [kept, O.a].join(",");
}
console.log(run("a"));
