const path = require("node:path");

function* range(start, end, step) {
  for (let value = start; value < end; value += step) {
    yield value;
  }
}

function memoize(fn) {
  const cache = new Map();
  return (key) => {
    if (!cache.has(key)) {
      cache.set(key, fn(key));
    }
    return cache.get(key);
  };
}

const fib = memoize((n) => (n < 2 ? n : fib(n - 1) + fib(n - 2)));
const unique = new Set(["red", "green", "red", "blue", "green"]);
const tag = Symbol("tag");
const tagged = { [tag]: "hidden", visible: "shown" };

async function delayed(label, value) {
  await null;
  return label + "=" + value;
}

async function main() {
  const results = await Promise.all([delayed("first", 1), delayed("second", 2)]);
  console.log("async", results.join(", "));
  try {
    await Promise.reject(new Error("rejected on purpose"));
  } catch (error) {
    console.log("caught", error.message);
  }
}

console.log("range", [...range(0, 20, 6)].join(" "));
console.log("fib", fib(30));
console.log("set", unique.size, [...unique].join("|"));
console.log("symbol", tagged[tag], Object.keys(tagged).length);
console.log("path", path.basename("/srv/data/report.txt", ".txt"), path.extname("x.tar.gz"));
console.log("regex", "2026-11-03".match(/(\d+)-(\d+)-(\d+)/).slice(1).join("/"));
main().then(() => console.log("finished"));
