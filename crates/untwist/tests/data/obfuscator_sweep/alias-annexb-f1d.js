function f(list) {
  var cache = {};
  var store = cache;
  for (var k = 0; k < list.length; k++) {
    cache[list[k]] = k;
    { function store(key) { return cache[key]; } }
  }
  console.log(typeof store, typeof store === "function" ? store("b") : store.b);
}
f(["a", "b"]);
