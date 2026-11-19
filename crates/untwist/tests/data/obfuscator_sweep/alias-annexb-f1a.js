function f() {
  var h = { abcde: "x" };
  var i = h;
  { function i() {} }
  console.log(typeof i, h.abcde);
}
f();
