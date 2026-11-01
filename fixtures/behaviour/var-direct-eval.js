function evaluated() {
  var x = 1;
  eval("var x = 2");
  return x;
}
console.log(evaluated());
