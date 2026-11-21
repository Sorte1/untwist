function f() {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); [arr0[0], arr0[1]] = [arr0[1], arr0[0]]; ({ length: globalThis.len9 } = arr0); }
  return arr0.join() + len9;
}
console.log(f());
