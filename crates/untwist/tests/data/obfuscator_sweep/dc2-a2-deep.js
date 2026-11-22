function g() {
  var x = 1;
  if ("b" !== "a") { let x = 4; try { for (;;) { if (x) { function x() {} } break; } } catch (e) {} console.log(x); }
  return typeof x;
}
console.log(g());
