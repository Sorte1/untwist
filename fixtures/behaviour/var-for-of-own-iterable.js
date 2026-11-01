function listed() {
  var out = [];
  for (var k of [k, 1]) {
    out.push(String(k));
  }
  return out.join(",");
}
console.log(listed());
