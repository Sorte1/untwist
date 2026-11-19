function f() {
  var h = {};
  h.abcde = "x";
  var i = h;
  if (h.abcde) function i() {}
  console.log(typeof i, h.abcde);
}
f();
