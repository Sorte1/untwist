function site() {
  var captured = 1;
  var reader = Function("return typeof captured");
  console.log("captured", reader());
}
site();
