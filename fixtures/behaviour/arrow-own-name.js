function compute() {
  var fact = function self(n) {
    return n <= 1 ? 1 : n * self(n - 1);
  };
  return fact(5);
}
console.log(compute());
