function f() {
  var o = { a: 0, b: "x" };
  o.a || console.log("zero", o.b);
}
f();
