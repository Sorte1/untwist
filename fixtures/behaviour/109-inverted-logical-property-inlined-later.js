function f(k) {
  var o = {};
  o.a = 0;
  o.b = "x";
  if (!(o.a && k)) { console.log("A", o.b); } else { console.log("B"); }
}
f(1);
