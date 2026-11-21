function f() {
  var arr0 = ["two", "three", "one"];
  var holder;
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  (holder = { updateCookie: function () { return true; } }).updateCookie() ? (r(3), arr0 = arr0.concat("z")) : r(2);
  return arr0.join();
}
console.log(f());
