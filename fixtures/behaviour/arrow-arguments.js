function outer() {
  var count = function () {
    return arguments.length;
  };
  return count(1, 2, 3);
}
console.log(outer(9));
