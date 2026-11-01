function make() {
  var made = [];
  for (var i = 0; i < 2; i++) {
    var seen = i;
    made.push(class {
      x = seen;
    });
  }
  var out = [];
  for (var k = 0; k < made.length; k++) {
    out.push(new made[k]().x);
  }
  return out.join(",");
}
console.log(make());
