function make() {
  var made = [];
  for (var i = 0; i < 3; i++) {
    var seen = i;
    made.push(function () {
      return seen;
    });
  }
  var out = [];
  for (var k = 0; k < made.length; k++) {
    out.push(made[k]());
  }
  return out.join(",");
}
console.log(make());
