var log = [];
function classify(value) {
  var kind = typeof value;
  if (kind === "string") {
    return "s:" + value.length;
  }
  if (kind === "number") {
    if (value > 10) return "big";
    return "small";
  }
  if (value === undefined) return "undef";
  return "other";
}
function tally(items) {
  var total = 0;
  var count = 0;
  var seen = {};
  for (var i = 0; i < items.length; i++) {
    var item = items[i];
    if (seen[item]) continue;
    seen[item] = true;
    count++;
    total += typeof item === "number" ? item : 0;
  }
  return { total: total, count: count };
}
function pick(obj, key, fallback) {
  var found = obj && obj[key];
  if (!found) {
    found = fallback;
    log.push("fallback " + key);
  }
  return found;
}
(function () {
  var state = { n: 0 };
  function bump() {
    state.n++;
    return state.n;
  }
  bump();
  bump();
  log.push("state " + state.n);
})();
var cache;
function getCache() {
  if (!cache) {
    cache = { hits: 0 };
  }
  cache.hits++;
  return cache;
}
getCache(); getCache();
log.push(classify("abc"), classify(42), classify(3), classify(undefined), classify(null));
var t = tally([1, 2, 2, "x", 3]);
log.push(t.total + "/" + t.count);
log.push(pick({ a: 1 }, "a", 9), pick(null, "b", 7));
log.push("hits " + getCache().hits);
var x = 5, y;
if (typeof y == "undefined") y = x * 2;
log.push(y);
console.log(log.join("\n"));
