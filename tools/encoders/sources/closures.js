function counter(start) {
  var n = start;
  return function () {
    n += 1;
    return n;
  };
}
var next = counter(10);
next();
console.log("next", next(), [1, 2, 3].map(function (x) { return x * 2; }).join(","));
