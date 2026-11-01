function sum(list) {
  var total = 0;
  for (var i = 0; i < list.length; i++) {
    var item = list[i] * 2;
    total += item;
  }
  return total;
}
function keys(object) {
  var out = [];
  for (var key in object) {
    out.push(key);
  }
  for (var value of [1, 2]) {
    out.push(String(value));
  }
  return out.join(",");
}
console.log(sum([1, 2, 3]), keys({ a: 1, b: 2 }));
