function build() {
  var Made = function () {};
  var made = eval("new Made()");
  return typeof made + ":" + Made();
}
console.log(build());
