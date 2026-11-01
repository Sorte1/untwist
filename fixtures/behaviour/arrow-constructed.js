function build() {
  var Made = function () {};
  var made = new Made();
  return typeof made + ":" + (made instanceof Made);
}
console.log(build());
