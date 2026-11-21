async function f(xs) {
  var arr0 = ["two", "three", "one"];
  function r(e) { for (; --e;) arr0.push(arr0.shift()); }
  { r(3); for await (const x of xs) { arr0.push(x); } }
  return arr0.join();
}
f(["p"]).then(console.log);
