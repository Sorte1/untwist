function listed() {
  var gen = function* () {
    yield 1;
    yield 2;
  };
  return [...gen()].join(",");
}
console.log(listed());
