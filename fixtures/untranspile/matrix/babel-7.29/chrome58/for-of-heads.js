"use strict";

const calls = [];
const handlers = [() => calls.push("a"), () => calls.push("b")];
for (const handler of new Set(handlers)) handler();
const counts = {};
for (const key of new Set(["x", "y", "x"])) counts[key] = (counts[key] || 0) + 1;
for (const entry of new Set([{
  list: [1, 2]
}, {
  list: [3]
}])) for (const n of entry.list) calls.push(n);
const removed = {
  a: 1,
  b: 2,
  c: 3
};
for (const key of new Set(["a", "c"])) delete removed[key];
let total = 0;
const weights = {
  p: 2,
  q: 5
};
for (const name of new Set(["p", "q", "r"])) total += weights[name] || 0;
for (const item of new Set([{
  run: k => calls.push(k)
}])) item.run("ran");
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
