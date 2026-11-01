function named(n) {
  switch (n) {
    case 0:
      var word = "zero";
      return word;
    case 1:
      word = "one";
      return word;
  }
  return "none";
}
console.log(named(0), named(1));
