function probe(flag) {
  if (flag) {
    var inner = 2;
  }
  return inner;
}
console.log(probe(process.argv.length > 0));
