function run() {
  var O = { a: 1, b: 2 };
  var seen = [];
  for (var key in O) {
    seen.push(key + "=" + O[key]);
  }
  return seen.join(",") + ";" + O.a;
}
console.log(run());
