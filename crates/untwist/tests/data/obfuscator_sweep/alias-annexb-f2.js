function f() {
  try { g(); } catch (e) { console.log(e.message); }
  const h = { abcde: "x" };
  const i = h;
  function g() { return i.abcde; }
  console.log(g(), h.abcde);
}
f();
