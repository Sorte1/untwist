const sym = Symbol("s");
const box = { a: 1, [sym]: 2 };
console.log(Reflect.ownKeys(box).length, typeof Reflect.ownKeys(box)[1], sym.description);
console.log([1, 2, 3].at(-1), Object.fromEntries([["x", 1]]).x, "abc".padStart(5, "-"), "abc".padEnd(5, "+"));
console.log([3, 1, 2].toSorted().join(), [1, 2, 3].findLast((x) => x < 3), Object.hasOwn(box, "a"));
console.log(Array.from(new Set([1, 1, 2])).length, [..."ab"].includes("b"), [NaN].includes(NaN));
console.log("a-b-c".replaceAll("-", "+"), [[1], [2, [3]]].flat(2).join(), "  x ".trimStart() + "|", " y  ".trimEnd() + "|");
console.log(Object.entries({ a: 1, b: 2 }).map(([k, v]) => k + v).join(), Object.values({ q: 5 })[0]);
console.log(new Map([[1, "a"]]).get(1), new WeakMap().has({}), Number.isInteger(5), Math.trunc(-4.5), Math.sign(-3));
console.log("x".repeat(3), "abc".startsWith("a"), "abc".endsWith("c"), "abc".includes("b"));
console.log([..."abc".matchAll(/./g)].length, [1, 2].flatMap((x) => [x, x]).length, "abc".at(-1));
console.log(JSON.stringify(Object.groupBy([1, 2, 3], (x) => (x % 2 ? "odd" : "even"))));
console.log(new URLSearchParams("a=1&b=2").get("b"), new URL("https://x.test/p?q=1").searchParams.get("q"));
console.log(structuredClone({ d: [1, 2] }).d.length, [1, 2, 3].with(0, 9).join(), [1, 2, 3].toReversed().join());
console.log(Array.from({ length: 3 }, (_, i) => i * 2).join(), Array.of(7).length, [1, 2, 3].fill(0, 1).join());
console.log(Object.assign({}, { z: 1 }).z, [1, 2, 3].find((x) => x > 1), [1, 2, 3].findIndex((x) => x > 1));
console.log("\u{20BB7}".codePointAt(0), String.fromCodePoint(134071) === "\u{20BB7}", new Error("e", { cause: "c" }).cause);
console.log([5, 1, 10].sort((x, y) => x - y).join(), [1, 2, 3].reverse().join(), [1, 2, 3, 4].slice(1, 3).join());
console.log(Number.parseFloat("1.5"), Number.EPSILON > 0, Math.hypot(3, 4), (1234.5678).toFixed(2));
console.log(JSON.stringify({ s: " " }), JSON.stringify([undefined, sym]), String(/a/giu.flags));
console.log(Object.getOwnPropertyNames(Symbol.prototype).includes("description"), typeof Symbol.asyncIterator);
console.log(new Set([1, 2]).union(new Set([3])).size, new Set([1, 2]).intersection(new Set([2])).size);
console.log(Promise.withResolvers !== undefined, typeof Array.prototype.findLastIndex, "ab".isWellFormed());
for (const [key, value] of new Map([["m", 1]])) console.log(key, value);
const { a, ...rest } = { a: 1, b: 2, c: 3 };
console.log(a, Object.keys(rest).join(), globalThis.Math === Math);
class Counter {
  #count = 1;
  static start = 2;
  get count() {
    return this.#count + Counter.start;
  }
}
console.log(new Counter().count);
async function later() {
  await null;
  return 7;
}
later().then((value) => console.log("async", value));
Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("e"))]).then((all) => console.log(all.map((one) => one.status).join()));
Promise.any([Promise.reject(new Error("x")), Promise.resolve(3)]).then((value) => console.log("any", value));
Promise.resolve(4)
  .finally(() => console.log("finally"))
  .then((value) => console.log("promise", value));
