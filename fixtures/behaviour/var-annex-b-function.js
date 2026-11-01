function shadowed() {
  var g = 1;
  var before = g;
  {
    function g() {}
  }
  return before + ":" + typeof g;
}
console.log(shadowed());
