function f() {
  var h = { abcde: "x" };
  var i = h;
  { function h() {} }
  console.log(typeof i, i.abcde, typeof h);
}
f();
