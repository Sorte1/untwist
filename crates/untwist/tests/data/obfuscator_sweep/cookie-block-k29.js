function f() {
  let arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); arr0 = arr0.slice(1); }
  return arr0[0];
}
console.log(f());
