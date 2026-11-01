function run(list) {
  var double = function (x) {
    return x * 2;
  };
  var out = [];
  list.forEach(function (v) {
    out.push(double(v) + 1);
  });
  return out.join(",");
}
console.log(run([1, 2, 3]));
