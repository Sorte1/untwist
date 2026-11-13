"use strict";

var calls = [];
var handlers = [() => calls.push("a"), () => calls.push("b")];
for (var handler of new Set(handlers)) handler();
var counts = {};
for (var key of new Set(["x", "y", "x"])) counts[key] = (counts[key] || 0) + 1;
for (var entry of new Set([{
  list: [1, 2]
}, {
  list: [3]
}])) for (var n of entry.list) calls.push(n);
var removed = {
  a: 1,
  b: 2,
  c: 3
};
for (var _key of new Set(["a", "c"])) delete removed[_key];
var total = 0;
var weights = {
  p: 2,
  q: 5
};
for (var name of new Set(["p", "q", "r"])) total += weights[name] || 0;
for (var item of new Set([{
  run: k => calls.push(k)
}])) item.run("ran");
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
