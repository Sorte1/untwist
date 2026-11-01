function run() {
  const key = "a";
  var O = { a: 1, b: 2 };
  var kept = O.b;
  delete O[key];
  return [kept, O.a].join(",");
}
console.log(run());
