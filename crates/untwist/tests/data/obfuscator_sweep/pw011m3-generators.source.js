function* range(start, end, step = 1) {
  for (let value = start; value < end; value += step) yield value;
}
function* pairs(list) {
  for (let i = 0; i < list.length; i++) {
    for (let j = i + 1; j < list.length; j++) yield [list[i], list[j]];
  }
}
const evens = [...range(0, 10, 2)];
console.log("evens", evens.join(" "));
for (const [a, b] of pairs(["x", "y", "z"])) console.log("pair", a + b);
const it = range(1, 3);
console.log(JSON.stringify(it.next()), JSON.stringify(it.next()), JSON.stringify(it.next()));
