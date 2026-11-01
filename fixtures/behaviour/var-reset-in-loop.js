function collect() {
  var out = [];
  for (var k = 0; k < 3; k++) {
    var x;
    if (k === 0) {
      x = 5;
    }
    out.push(String(x));
  }
  return out.join(",");
}
console.log(collect());
